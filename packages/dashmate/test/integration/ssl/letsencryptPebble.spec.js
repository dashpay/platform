import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import Docker from 'dockerode';
import { asValue } from 'awilix';
import createDIContainer from '../../../src/createDIContainer.js';
import Config from '../../../src/config/Config.js';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../src/config/configFile/ConfigFileJsonRepository.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import isCertificatePairInstalled from '../../../src/ssl/letsencrypt/isCertificatePairInstalled.js';
import renewCertificate from '../../../src/helper/renewCertificate.js';

/**
 * Obtain a certificate from a real ACME server.
 *
 * Everything below the ACME directory URL is production code: the lego image,
 * its arguments, the HTTP-01 challenge it answers, and the files it produces.
 * Only the certificate authority is swapped, for Pebble - the server Let's
 * Encrypt tests its own implementation against.
 *
 * Pebble rather than a stub because Dashmate identifies a node by its external
 * IP, and an IP certificate is what has to work. Pebble implements the IP
 * identifier extension and ships the same `shortlived` profile name Let's
 * Encrypt requires for them, so the production arguments run unchanged.
 */
const PEBBLE_IMAGE = 'ghcr.io/letsencrypt/pebble:latest';

// Pebble presents a certificate for `pebble`, `localhost` and 127.0.0.1 only,
// so it has to be reached by a name it was issued for.
const PEBBLE_HOSTNAME = 'pebble';
const PEBBLE_ACME_PORT = 14000;

// lego asks for a certificate covering a fixed address and Pebble connects back
// to it, so both need addresses known before they start. Docker only honours a
// requested address on a network whose subnet was configured explicitly, so the
// pool cannot simply be left to it - instead a candidate is tried and the next
// one used if the machine is already occupying it.
const CANDIDATE_SUBNETS = [
  '172.29.0.0/24',
  '172.30.0.0/24',
  '172.31.0.0/24',
  '10.229.0.0/24',
  '10.230.0.0/24',
];

const DOCKER_SUBNET_OVERLAP_MESSAGE = 'Pool overlaps with other one on this address space';

/**
 * @param {Error & {reason?: string, json?: {message?: string}}} error
 * @return {boolean}
 */
function isDockerSubnetOverlapError(error) {
  return [error.message, error.reason, error.json && error.json.message]
    .some((message) => typeof message === 'string'
      && message.includes(DOCKER_SUBNET_OVERLAP_MESSAGE));
}

/**
 * Create the test network from the first candidate Docker accepts.
 *
 * @param {Docker} docker
 * @param {string} networkName
 * @param {string[]} candidateSubnets
 * @return {Promise<{network: Object, subnet: string}>}
 */
async function createNetworkFromCandidateSubnets(docker, networkName, candidateSubnets) {
  let lastError;

  for (const subnet of candidateSubnets) {
    try {
      const network = await docker.createNetwork({
        Name: networkName,
        IPAM: { Config: [{ Subnet: subnet }] },
      });

      return { network, subnet };
    } catch (e) {
      if (!isDockerSubnetOverlapError(e)) {
        throw e;
      }

      lastError = e;
    }
  }

  throw new Error('No candidate subnet was free for the ACME test network:'
    + ` ${lastError && lastError.message}`);
}

describe('Pebble candidate network selection', () => {
  const overlapErrors = [
    ['message', () => new Error(DOCKER_SUBNET_OVERLAP_MESSAGE)],
    ['reason', () => Object.assign(new Error('network creation failed'), {
      reason: DOCKER_SUBNET_OVERLAP_MESSAGE,
    })],
    ['json.message', () => Object.assign(new Error('network creation failed'), {
      json: { message: DOCKER_SUBNET_OVERLAP_MESSAGE },
    })],
  ];

  overlapErrors.forEach(([field, createOverlapError]) => {
    it(`should try the next candidate when Docker reports overlap in ${field}`, async function it() {
      const network = {};
      const docker = {
        createNetwork: this.sinon.stub(),
      };
      docker.createNetwork.onFirstCall().rejects(createOverlapError());
      docker.createNetwork.onSecondCall().resolves(network);

      const result = await createNetworkFromCandidateSubnets(
        docker,
        'acme-test',
        ['172.29.0.0/24', '172.30.0.0/24'],
      );

      expect(result).to.deep.equal({ network, subnet: '172.30.0.0/24' });
    });
  });

  it('should not hide an unrelated Docker failure', async function it() {
    const permissionError = new Error('permission denied');
    const docker = {
      createNetwork: this.sinon.stub(),
    };
    docker.createNetwork.onFirstCall().rejects(permissionError);
    docker.createNetwork.onSecondCall().resolves({});

    await expect(createNetworkFromCandidateSubnets(
      docker,
      'acme-test',
      ['172.29.0.0/24', '172.30.0.0/24'],
    )).to.be.rejectedWith(permissionError);

    expect(docker.createNetwork).to.have.been.calledOnce();
  });

  it('should report exhaustion after every candidate overlaps', async function it() {
    const docker = {
      createNetwork: this.sinon.stub().rejects(new Error(DOCKER_SUBNET_OVERLAP_MESSAGE)),
    };

    await expect(createNetworkFromCandidateSubnets(
      docker,
      'acme-test',
      ['172.29.0.0/24', '172.30.0.0/24'],
    )).to.be.rejectedWith('No candidate subnet was free for the ACME test network');

    expect(docker.createNetwork).to.have.been.calledTwice();
  });
});

describe('Let\'s Encrypt certificate against a local ACME server', function main() {
  // `lego renew` sleeps a random delay of up to about eight minutes when the
  // authority's renewalInfo endpoint says renewal is not yet due, which the
  // renewal case below always hits: the certificate it renews was issued
  // moments earlier. The budget covers that sleep rather than racing it.
  this.timeout(15 * 60 * 1000);

  const docker = new Docker();
  const networkName = `dashmate-acme-test-${crypto.randomBytes(4).toString('hex')}`;

  let network;
  let pebbleIp;
  let legoIp;
  let pebbleContainer;
  let pebbleDir;
  let homeDir;
  let container;
  let config;
  let sslDir;

  /**
   * Pebble is distroless, so its CA and default config are read out of the
   * image rather than from a shell inside it.
   *
   * @param {string} destination
   */
  async function extractPebbleFixtures(destination) {
    const created = await docker.createContainer({ Image: PEBBLE_IMAGE });

    try {
      const archive = await created.getArchive({ path: '/test' });

      await new Promise((resolve, reject) => {
        const write = fs.createWriteStream(path.join(destination, 'test.tar'));
        archive.pipe(write);
        archive.on('error', reject);
        write.on('finish', resolve);
        write.on('error', reject);
      });
    } finally {
      await created.remove({ force: true });
    }

    const { execFileSync } = await import('child_process');
    execFileSync('tar', ['-xf', path.join(destination, 'test.tar'), '-C', destination]);
  }

  before(async () => {
    await new Promise((resolve, reject) => {
      docker.pull(PEBBLE_IMAGE, (err, stream) => {
        if (err) {
          reject(err);
          return;
        }
        docker.modem.followProgress(stream, (e) => (e ? reject(e) : resolve()));
      });
    });

    pebbleDir = fs.mkdtempSync(path.join(HomeDir.createTemp().getPath(), 'pebble-'));

    await extractPebbleFixtures(pebbleDir);

    // Validate against the port the production lego arguments serve on, so
    // `--http.port :80` is exercised rather than replaced.
    const pebbleConfigPath = path.join(pebbleDir, 'pebble-config.json');
    const pebbleConfig = JSON.parse(
      fs.readFileSync(path.join(pebbleDir, 'test', 'config', 'pebble-config.json'), 'utf8'),
    );
    pebbleConfig.pebble.httpPort = 80;
    fs.writeFileSync(pebbleConfigPath, JSON.stringify(pebbleConfig), 'utf8');

    const selected = await createNetworkFromCandidateSubnets(
      docker,
      networkName,
      CANDIDATE_SUBNETS,
    );
    network = selected.network;

    const [prefix] = selected.subnet.split('/');
    const octets = prefix.split('.');

    // .1 is the gateway Docker assigns itself.
    pebbleIp = [...octets.slice(0, 3), '2'].join('.');
    legoIp = [...octets.slice(0, 3), '3'].join('.');

    pebbleContainer = await docker.createContainer({
      Image: PEBBLE_IMAGE,
      Cmd: ['-config', '/test/config/pebble-config.json'],
      // Without this Pebble sleeps before validating, for no benefit here.
      Env: ['PEBBLE_VA_NOSLEEP=1'],
      HostConfig: {
        AutoRemove: true,
        NetworkMode: networkName,
        Binds: [`${pebbleConfigPath}:/test/config/pebble-config.json:ro`],
      },
      NetworkingConfig: {
        EndpointsConfig: {
          [networkName]: {
            IPAMConfig: { IPv4Address: pebbleIp },
            Aliases: [PEBBLE_HOSTNAME],
          },
        },
      },
    });

    await pebbleContainer.start();

    homeDir = HomeDir.createTemp();

    container = await createDIContainer({});
    container.resolve('homeDir').change(homeDir);
    container.register({
      legoCaCertificatePath: asValue(
        path.join(pebbleDir, 'test', 'certs', 'pebble.minica.pem'),
      ),
      legoContainerOptions: asValue({
        HostConfig: {
          NetworkMode: networkName,
          // The challenge is served over the test network, so nothing needs to
          // reach the host - and binding its port 80 would need root.
          PortBindings: {},
        },
        NetworkingConfig: {
          EndpointsConfig: {
            [networkName]: { IPAMConfig: { IPv4Address: legoIp } },
          },
        },
      }),
    });

    config = getBaseConfigFactory(homeDir)();
    // lego asks for a certificate covering this address, and Pebble connects
    // back to it to validate - so it has to be the address lego answers on.
    config.set('externalIp', legoIp);
    config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', 'test@dash.org');
    config.set(
      'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl',
      `https://${PEBBLE_HOSTNAME}:${PEBBLE_ACME_PORT}/dir`,
    );

    sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
  });

  after(async () => {
    if (pebbleContainer) {
      await pebbleContainer.stop().catch(() => {});
    }

    if (network) {
      await network.remove().catch(() => {});
    }

    if (homeDir) {
      homeDir.remove();
    }
  });

  /**
   * @param {string} certificatePath
   * @param {string} keyPath
   * @return {boolean}
   */
  function isMatchingPair(certificatePath, keyPath) {
    const certificate = new crypto.X509Certificate(fs.readFileSync(certificatePath));

    return certificate.checkPrivateKey(
      crypto.createPrivateKey(fs.readFileSync(keyPath)),
    );
  }

  it('should obtain a certificate for the external IP and install it for the gateway', async () => {
    const obtainLetsEncryptCertificateTask = container.resolve('obtainLetsEncryptCertificateTask');

    await obtainLetsEncryptCertificateTask(config).run({ force: true });

    const certificatePath = path.join(sslDir, 'bundle.crt');
    const keyPath = path.join(sslDir, 'private.key');

    expect(fs.existsSync(certificatePath)).to.be.true();
    expect(fs.existsSync(keyPath)).to.be.true();

    // A certificate and key that do not belong together is the failure the
    // gateway cannot recover from, and it is invisible in file listings.
    expect(isMatchingPair(certificatePath, keyPath)).to.be.true();

    // The certificate has to cover the address the node is reached on. Dashmate
    // passes --disable-cn, so the IP is only ever in the SAN.
    const certificate = new crypto.X509Certificate(fs.readFileSync(certificatePath));
    expect(certificate.subjectAltName).to.equal(`IP Address:${legoIp}`);

    // A private key readable by anyone on the host is the regression this
    // guards - it is not visible in any command output.
    /* eslint-disable no-bitwise */
    expect(fs.statSync(keyPath).mode & 0o777).to.equal(0o600);
    expect(fs.statSync(certificatePath).mode & 0o777).to.equal(0o644);
    /* eslint-enable no-bitwise */

    // The provider is recorded only once usable files are in place, so a node
    // never claims SSL it cannot serve.
    expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
    expect(config.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');

    // Nothing may be left in the directory the gateway reads.
    expect(fs.readdirSync(sslDir).filter((f) => f.includes('.tmp-'))).to.have.lengthOf(0);
  });

  it('should leave an installed certificate and the configuration untouched', async () => {
    const obtainLetsEncryptCertificateTask = container.resolve('obtainLetsEncryptCertificateTask');

    const certificatePath = path.join(sslDir, 'bundle.crt');
    const keyPath = path.join(sslDir, 'private.key');
    const before = {
      certificate: fs.readFileSync(certificatePath),
      key: fs.readFileSync(keyPath),
    };

    // An operator hardening the key further must not have it undone by a
    // renewal check that had nothing to do.
    fs.chmodSync(keyPath, 0o400);
    config.markAsSaved();

    await obtainLetsEncryptCertificateTask(config).run({});

    expect(fs.readFileSync(certificatePath)).to.deep.equal(before.certificate);
    expect(fs.readFileSync(keyPath)).to.deep.equal(before.key);
    // eslint-disable-next-line no-bitwise
    expect(fs.statSync(keyPath).mode & 0o777).to.equal(0o400);

    // Rewriting an unchanged configuration is what made read-only commands
    // clobber concurrent edits, and a renewal check runs unattended.
    expect(config.isChanged()).to.be.false();
  });

  /**
   * Nothing prompts for a contact address any more, so every fresh setup and
   * every migration from another provider issues without one. If contactless
   * issuance does not work, the feature does not work - which is why this is a
   * gate rather than a nice-to-have.
   *
   * The client half is already measured: lego does not require --email and
   * substitutes noemail@example.com as a local directory name. What is proved
   * here is the CA half - that registration without a contact is accepted and
   * the certificate that comes back is the same certificate.
   */
  describe('issuance without a contact address', () => {
    /**
     * @param {string} name
     * @param {string|null} email
     * @return {Config}
     */
    function createConfig(name, email) {
      const created = new Config(name, getBaseConfigFactory(homeDir)().getOptions());

      created.set('externalIp', legoIp);
      created.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', email);
      created.set(
        'platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl',
        `https://${PEBBLE_HOSTNAME}:${PEBBLE_ACME_PORT}/dir`,
      );

      return created;
    }

    /**
     * @param {Config} target
     * @return {{certificate: crypto.X509Certificate, paired: boolean, accounts: string[]}}
     */
    function inspect(target) {
      const dir = homeDir.joinPath(target.getName(), 'platform', 'gateway', 'ssl');
      const legoDir = homeDir.joinPath(target.getName(), 'platform', 'gateway', 'lego');
      const bundlePath = path.join(dir, 'bundle.crt');
      const keyPath = path.join(dir, 'private.key');

      return {
        certificate: new crypto.X509Certificate(fs.readFileSync(bundlePath)),
        paired: isCertificatePairInstalled(
          path.join(legoDir, 'certificates', `${legoIp}.crt`),
          path.join(legoDir, 'certificates', `${legoIp}.key`),
          bundlePath,
          keyPath,
        ),
        accounts: fs.readdirSync(path.join(legoDir, 'accounts'), { recursive: true })
          .map((entry) => entry.toString()),
      };
    }

    let contactless;
    let withContact;

    // The renewal below refuses to run unless the provider says letsencrypt, so
    // a test that leaves it changed - including one that fails part way through
    // - would take the renewal down with it and hide which of the two broke.
    afterEach(() => {
      if (contactless) {
        contactless.set('platform.gateway.ssl.provider', 'letsencrypt');
      }
    });

    before(async () => {
      const obtainLetsEncryptCertificateTask = container.resolve('obtainLetsEncryptCertificateTask');

      contactless = createConfig('contactless', null);
      withContact = createConfig('withcontact', 'operator@example.org');

      await obtainLetsEncryptCertificateTask(contactless).run({ force: true });
      await obtainLetsEncryptCertificateTask(withContact).run({ force: true });
    });

    it('should produce the same certificate with and without a contact address', () => {
      const a = inspect(contactless);
      const b = inspect(withContact);

      // Same identifier, same validity window length, same subject alternative
      // name. A contact address buys nothing from the authority.
      expect(a.certificate.subjectAltName).to.equal(`IP Address:${legoIp}`);
      expect(b.certificate.subjectAltName).to.equal(a.certificate.subjectAltName);

      const window = (certificate) => new Date(certificate.validTo).getTime()
        - new Date(certificate.validFrom).getTime();

      expect(window(a.certificate)).to.equal(window(b.certificate));

      // Both have to be installed as a matching pair, or the gateway cannot
      // serve either of them.
      expect(a.paired).to.be.true();
      expect(b.paired).to.be.true();
    });

    it('should record the provider for a node that has no contact address', () => {
      expect(contactless.get('platform.gateway.ssl.enabled')).to.be.true();
      expect(contactless.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');
      expect(contactless.get('platform.gateway.ssl.providerConfigs.letsencrypt.email')).to.be.null();
    });

    // The account directory lego uses is named after the contact address, so a
    // contactless node's account lives somewhere else entirely. `lego renew`
    // needs the account that issued, which makes this the half of contactless
    // operation that issuance alone does not prove.
    it('should keep the two accounts apart on disk', () => {
      expect(inspect(contactless).accounts.some((entry) => entry.includes('noemail@example.com')))
        .to.be.true();
      expect(inspect(withContact).accounts.some((entry) => entry.includes('operator@example.org')))
        .to.be.true();
    });

    // A node that already has an address on file must keep using the account
    // that address names. Nothing may quietly move it: a new account means a
    // new account key and a reset failed-authorization budget, spent against
    // the per-address registration limit.
    //
    // Issued rather than renewed, so this does not pay lego's renewal delay
    // twice. The renewal path is covered below, and the property under test -
    // which account directory the address resolves to - is the same either way.
    it('should reissue for a node with a contact address against its original account', async () => {
      const obtainLetsEncryptCertificateTask = container.resolve('obtainLetsEncryptCertificateTask');
      const accountsBefore = inspect(withContact).accounts;
      const serialBefore = inspect(withContact).certificate.serialNumber;

      await obtainLetsEncryptCertificateTask(withContact).run({ force: true });

      const after = inspect(withContact);

      expect(after.certificate.serialNumber).to.not.equal(serialBefore);
      expect(after.accounts).to.deep.equal(accountsBefore);
      expect(withContact.get('platform.gateway.ssl.providerConfigs.letsencrypt.email'))
        .to.equal('operator@example.org');
    });

    // The window between installing the pair and saving the provider. Left as
    // a warning this never repairs itself - the helper keeps renewing the old
    // provider while the installed six-day certificate runs out - so it has to
    // block, and the block has to name a repair that needs no new certificate.
    it('should detect a switch interrupted before the provider was saved', () => {
      const checkGatewayCertificate = container.resolve('checkGatewayCertificate');

      expect(checkGatewayCertificate(contactless).status).to.equal('CHECKS_PASSED');

      // Exactly what a kill between the two steps leaves behind: the pair lego
      // produced is installed for the gateway, the setting still names the
      // provider it was switched away from.
      contactless.set('platform.gateway.ssl.provider', 'zerossl');

      const verdict = checkGatewayCertificate(contactless);

      expect(verdict.status).to.equal('INVALID');
      expect(verdict.reasons.map(({ code }) => code)).to.deep.equal(['SWITCH_INCOMPLETE']);
    });

    // Renewal is where a missing account would surface, and it runs unattended
    // inside the helper - the one place a failure goes unnoticed for months.
    it('should renew a contactless certificate through the helper entry point', async () => {
      const obtainLetsEncryptCertificateTask = container.resolve('obtainLetsEncryptCertificateTask');
      const before = inspect(contactless).certificate.serialNumber;

      const configFile = new ConfigFile(
        [contactless],
        '4.2.0',
        'abcdef12',
        contactless.getName(),
        null,
      );
      const configFileRepository = new ConfigFileJsonRepository(
        (data) => data,
        homeDir,
        () => null,
      );
      configFileRepository.write(configFile);

      const { renewed } = await renewCertificate({
        configName: contactless.getName(),
        provider: 'letsencrypt',
        // Well past the certificate's own six-day life, so renewal is due.
        expirationDays: 60,
        obtainCertificateTask: obtainLetsEncryptCertificateTask,
        configFileRepository,
        writeConfigTemplates: () => {},
      });

      expect(renewed).to.be.true();

      const after = inspect(contactless);
      expect(after.certificate.serialNumber).to.not.equal(before);
      expect(after.certificate.subjectAltName).to.equal(`IP Address:${legoIp}`);
      expect(after.paired).to.be.true();
    });
  });
});

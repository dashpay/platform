import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import Docker from 'dockerode';
import { asValue } from 'awilix';
import createDIContainer from '../../../src/createDIContainer.js';
import HomeDir from '../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';

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
  this.timeout(5 * 60 * 1000);

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
});

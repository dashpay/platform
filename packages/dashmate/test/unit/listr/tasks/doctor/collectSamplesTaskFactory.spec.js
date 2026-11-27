import crypto from 'crypto';
import fs from 'fs';
import os from 'os';
import path from 'path';
import tls from 'node:tls';
import zlib from 'zlib';
import { Listr } from 'listr2';
import si from 'systeminformation';
import getBaseConfigFactory from '../../../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../../../src/config/HomeDir.js';
import createCertificateForTest from '../../../../../src/test/createCertificateForTest.js';
import analyseConfigFactory from '../../../../../src/doctor/analyse/analyseConfigFactory.js';
import analyseCoreFactory from '../../../../../src/doctor/analyse/analyseCoreFactory.js';
import analyseGatewayCertificateFactory from '../../../../../src/doctor/analyse/analyseGatewayCertificateFactory.js';
import analyseServiceContainersFactory from '../../../../../src/doctor/analyse/analyseServiceContainersFactory.js';
import archiveSamples from '../../../../../src/doctor/archiveSamples.js';
import { SEVERITY } from '../../../../../src/doctor/Prescription.js';
import Samples from '../../../../../src/doctor/Samples.js';
import unarchiveSamplesFactory from '../../../../../src/doctor/unarchiveSamplesFactory.js';
import { getOperatorIdentity } from '../../../../../src/util/maskOperatorIdentity.js';
import collectSamplesTaskFactory from '../../../../../src/listr/tasks/doctor/collectSamplesTaskFactory.js';
import Certificate from '../../../../../src/ssl/zerossl/Certificate.js';
import generateCsr from '../../../../../src/ssl/zerossl/generateCsr.js';
import checkGatewayCertificateFactory from '../../../../../src/ssl/checkGatewayCertificateFactory.js';
import validateZeroSslCertificateFactory, { ERRORS as ZEROSSL_ERRORS } from '../../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import validateLetsEncryptCertificateFactory from '../../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import providers from '../../../../../src/status/providers.js';
import RenewalRecord from '../../../../../src/ssl/renewalRecord/RenewalRecord.js';
import RenewalRecordRepository from '../../../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import { PACKAGE_ROOT_DIR } from '../../../../../src/constants.js';
import deriveTenderdashNodeId from '../../../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../../../src/tenderdash/generateTenderdashNodeKey.js';

const EXTERNAL_IP = '198.51.100.7';

/**
 * Read a Core RPC response captured from a real Dash Core node
 *
 * @param {string} name
 * @return {Object}
 */
function readCoreRpcFixture(name) {
  return JSON.parse(fs.readFileSync(
    path.join(PACKAGE_ROOT_DIR, 'test/fixtures/coreRpc/24.0.3', name),
    'utf8',
  ));
}

/**
 * Format a date the way the ZeroSSL API reports certificate dates
 *
 * @param {Date} date
 * @return {string}
 */
function toZeroSslDate(date) {
  const pad = (number) => String(number).padStart(2, '0');

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} `
    + `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

/**
 * @param {number} days
 * @return {Date}
 */
function daysFromNow(days) {
  const date = new Date();

  date.setDate(date.getDate() + days);

  return date;
}

describe('collectSamplesTaskFactory', () => {
  let homeDir;
  let config;
  let getCertificate;
  let collectSamplesTask;
  let analyseConfig;
  let samples;
  let dockerCompose;
  let rpcClient;
  let originalUser;

  /**
   * Run the sample collection the same way the doctor command does: as a subtask
   * of a parent list, so the parent's renderer applies.
   *
   * @return {Promise<void>}
   */
  async function collectSamples() {
    const tasks = new Listr(
      [{ task: () => collectSamplesTask(config) }],
      { renderer: 'silent' },
    );

    await tasks.run({ samples });
  }

  beforeEach(async function beforeEach() {
    originalUser = process.env.USER;
    homeDir = HomeDir.createTemp();

    config = getBaseConfigFactory()();

    config.set('externalIp', EXTERNAL_IP);
    config.set('platform.enable', true);
    config.set('platform.gateway.ssl.enabled', true);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'a'.repeat(32));
    config.set('platform.gateway.ssl.providerConfigs.zerossl.id', 'b'.repeat(32));

    // The ZeroSSL validator inspects the certificate files on disk
    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');

    fs.mkdirSync(sslDir, { recursive: true });
    // Dashmate writes the key accessible to its owner only, and doctor reports
    // one that is not - so the fixture has to be what a healthy node has: an
    // RSA key and the certificate request renewal sends for it
    const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    });
    fs.writeFileSync(path.join(sslDir, 'csr.pem'), await generateCsr({ publicKey, privateKey }, EXTERNAL_IP), 'utf8');
    fs.writeFileSync(path.join(sslDir, 'private.key'), privateKey, {
      encoding: 'utf8',
      mode: 0o600,
    });
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), 'bundle', 'utf8');

    getCertificate = this.sinon.stub();

    this.sinon.stub(providers.mnowatch, 'checkPortStatus').resolves('OPEN');

    this.sinon.stub(global, 'fetch').resolves({
      json: async () => ({}),
      text: async () => 'metrics_sample 1',
    });

    dockerCompose = {
      throwErrorIfNotInstalled: this.sinon.stub().resolves(),
      inspectService: this.sinon.stub().resolves({}),
      logs: this.sinon.stub().resolves({ out: '', err: '' }),
    };

    rpcClient = {
      getBestChainLock: this.sinon.stub().resolves({ result: {} }),
      quorum: this.sinon.stub().resolves({ result: {} }),
      getBlockchainInfo: this.sinon.stub().resolves({ result: {} }),
      getPeerInfo: this.sinon.stub().resolves({ result: {} }),
      mnsync: this.sinon.stub().resolves({ result: {} }),
      masternode: this.sinon.stub().resolves({ result: {} }),
    };

    collectSamplesTask = collectSamplesTaskFactory(
      dockerCompose,
      this.sinon.stub().returns(rpcClient),
      this.sinon.stub().resolves('127.0.0.1'),
      this.sinon.stub().returns({ request: this.sinon.stub().resolves({}) }),
      this.sinon.stub().resolves([]),
      this.sinon.stub().resolves({}),
      homeDir,
      validateZeroSslCertificateFactory(homeDir, getCertificate),
      this.sinon.stub().resolves({}),
      checkGatewayCertificateFactory(homeDir),
      new RenewalRecordRepository(homeDir),
    );

    analyseConfig = analyseConfigFactory();

    samples = new Samples();
  });

  afterEach(() => {
    homeDir.remove();

    // The masking cases below mutate and delete USER, and one of them leaves it
    // deleted for every later test in the process otherwise.
    if (originalUser === undefined) {
      delete process.env.USER;
    } else {
      process.env.USER = originalUser;
    }
  });

  it('should report a problem for a ZeroSSL certificate that expired months ago', async () => {
    const expiredAt = daysFromNow(-180);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-270)),
      expires: toZeroSslDate(expiredAt),
    }));

    await collectSamples();

    expect(samples.getServiceInfo('gateway', 'ssl').error)
      .to.equal(ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON);

    const problems = analyseConfig(samples);

    const sslProblem = problems
      .find((problem) => problem.getDescription().includes('ZeroSSL certificate expires at'));

    expect(sslProblem).to.exist();
    expect(sslProblem.getSeverity()).to.equal(SEVERITY.HIGH);
  });

  // Renewal is due, so the sample carries the request renewal would send.
  it('should not put a private key held in the certificate request in the archive', async () => {
    const csrFilePath = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl', 'csr.pem');
    const privateKey = fs.readFileSync(
      homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl', 'private.key'),
      'utf8',
    );
    fs.appendFileSync(csrFilePath, privateKey);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-270)),
      expires: toZeroSslDate(daysFromNow(-180)),
    }));

    await collectSamples();

    const sample = samples.getServiceInfo('gateway', 'ssl');

    expect(sample.error).to.equal(ZEROSSL_ERRORS.CSR_FILE_IS_NOT_PRESENT);
    expect(JSON.stringify(sample)).to.not.include('PRIVATE KEY');
  });

  it('should not report a problem for a valid ZeroSSL certificate', async () => {
    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-1)),
      expires: toZeroSslDate(daysFromNow(89)),
    }));

    await collectSamples();

    expect(samples.getServiceInfo('gateway', 'ssl').error).to.be.undefined();

    expect(analyseConfig(samples)).to.be.empty();
  });

  // Doctor archives are the artefact operators hand to support, and a
  // certificate problem names the file it could not read - an absolute path
  // under the operator's home directory. Every neighbouring certificate branch
  // masks the username before storing; this one has to as well.
  // Read from the operating system rather than the environment, because the
  // point of the test is that masking still happens when the environment does
  // not say who is running.
  const operator = os.userInfo().username;

  [
    ['with USER set', operator],
    ['with USER unset', undefined],
  ].forEach(([name, userValue]) => {
    it(`should not put the operator's username in a shared archive, ${name}`, async function it() {
      if (userValue === undefined) {
        delete process.env.USER;
      } else {
        process.env.USER = userValue;
      }

      const leakyPath = `/Users/${operator}/.dashmate/base/platform/gateway/ssl/bundle.crt`;

      const task = collectSamplesTaskFactory(
        dockerCompose,
        this.sinon.stub().returns(rpcClient),
        this.sinon.stub().resolves('127.0.0.1'),
        this.sinon.stub().returns({ request: this.sinon.stub().resolves({}) }),
        this.sinon.stub().resolves([]),
        this.sinon.stub().resolves({}),
        homeDir,
        validateZeroSslCertificateFactory(homeDir, getCertificate),
        this.sinon.stub().resolves({}),
        () => ({
          status: 'INVALID',
          reasons: [{
            code: 'BUNDLE_MISSING',
            message: `dashmate could not find the certificate bundle at ${leakyPath}`,
          }],
          warnings: [],
          skipped: [],
          provider: 'zerossl',
          installed: null,
          expiresInDays: null,
        }),
        new RenewalRecordRepository(homeDir),
      );

      getCertificate.resolves(new Certificate({
        id: 'certificate-id',
        common_name: EXTERNAL_IP,
        status: 'issued',
        created: toZeroSslDate(daysFromNow(-1)),
        expires: toZeroSslDate(daysFromNow(89)),
      }));

      await new Listr([{ task: () => task(config) }], { renderer: 'silent' }).run({ samples });

      const serialised = JSON.stringify(samples.getServiceInfo('gateway', 'installedCertificate'));

      // The path is still there - it is what makes the problem actionable - but
      // the name in it is not.
      expect(serialised).to.contain('bundle.crt');
      expect(serialised).to.not.contain(operator);
    });
  });

  // Doctor will only advise loading the file over a working served certificate
  // when the checks that judged the file describe the same file the probe
  // measured. The two fingerprints come from different selectors - the probe
  // takes the first non-CA block, the checks take the block matching the
  // private key - so that they agree for an ordinary pair is a property worth
  // holding, not an assumption. If it ever stops holding the advice silently
  // stops being given.
  it('should record the same certificate in the verdict and the wire sample', async () => {
    const { cert, key } = createCertificateForTest({ ip: EXTERNAL_IP, days: 30 });
    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');

    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), cert, 'utf8');
    fs.writeFileSync(path.join(sslDir, 'private.key'), key, 'utf8');

    const server = tls.createServer({ cert, key }, (socket) => socket.end());
    const liveSockets = [];
    server.on('secureConnection', (socket) => liveSockets.push(socket));

    await new Promise((resolve) => { server.listen(0, '127.0.0.1', resolve); });

    config.set('platform.gateway.listeners.dapiAndDrive.port', server.address().port);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-1)),
      expires: toZeroSslDate(daysFromNow(89)),
    }));

    try {
      await collectSamples();
    } finally {
      liveSockets.forEach((socket) => socket.destroy());
      await new Promise((resolve) => { server.close(resolve); });
    }

    const installed = samples.getServiceInfo('gateway', 'installedCertificate');
    const servedSample = samples.getServiceInfo('gateway', 'servedCertificate');

    expect(installed.fingerprint256).to.be.a('string');
    expect(servedSample.onDisk.fingerprint256).to.equal(installed.fingerprint256);
  });

  it('should collect the certificate the gateway actually serves', async () => {
    const { cert, key } = createCertificateForTest({ ip: EXTERNAL_IP, days: 30 });

    const server = tls.createServer({ cert, key }, (socket) => socket.end());
    const liveSockets = [];

    server.on('secureConnection', (socket) => liveSockets.push(socket));

    await new Promise((resolve) => {
      server.listen(0, '127.0.0.1', resolve);
    });

    // The gateway's own bundle is the certificate the server presents, so disk and wire agree
    fs.writeFileSync(
      path.join(homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl'), 'bundle.crt'),
      cert,
      'utf8',
    );

    config.set('platform.gateway.listeners.dapiAndDrive.port', server.address().port);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-1)),
      expires: toZeroSslDate(daysFromNow(89)),
    }));

    try {
      await collectSamples();
    } finally {
      liveSockets.forEach((socket) => socket.destroy());
      await new Promise((resolve) => {
        server.close(resolve);
      });
    }

    const servedCertificate = samples.getServiceInfo('gateway', 'servedCertificate');

    expect(servedCertificate.state).to.equal('served');
    expect(servedCertificate.identityVerified).to.be.true();
    expect(servedCertificate.matchesOnDisk).to.be.true();
    expect(samples.getServiceInfo('gateway', 'validationHttpPort')).to.equal('OPEN');
  });

  // The certificate check reads the API key to call ZeroSSL with, and hands it
  // back with the rest of what it looked at. Doctor never needs the key itself -
  // whether it is set is already in the check's verdict.
  it('should not collect the ZeroSSL API key', async () => {
    const apiKey = 'collectedZeroSslApiKey'.padEnd(32, '7');

    config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', apiKey);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-1)),
      expires: toZeroSslDate(daysFromNow(89)),
    }));

    await collectSamples();

    expect(samples.getServiceInfo('gateway', 'ssl').data).to.include.keys('bundleFilePath');
    expect(samples.getServiceInfo('gateway', 'ssl').data).to.not.have.property('apiKey');
    expect(JSON.stringify(samples.getServices())).to.not.contain(apiKey);
  });

  it('should collect metrics as text rather than an unresolved promise', async () => {
    config.set('platform.gateway.metrics.enabled', true);

    getCertificate.resolves(new Certificate({
      id: 'certificate-id',
      common_name: EXTERNAL_IP,
      status: 'issued',
      created: toZeroSslDate(daysFromNow(-1)),
      expires: toZeroSslDate(daysFromNow(89)),
    }));

    await collectSamples();

    expect(samples.getServiceInfo('gateway', 'metrics')).to.equal('metrics_sample 1');
  });

  describe('Core status', () => {
    let masternodeStatus;
    let mnsyncStatus;

    beforeEach(() => {
      getCertificate.resolves(new Certificate({
        id: 'certificate-id',
        common_name: EXTERNAL_IP,
        status: 'issued',
        created: toZeroSslDate(daysFromNow(-1)),
        expires: toZeroSslDate(daysFromNow(89)),
      }));

      masternodeStatus = readCoreRpcFixture('masternode-status-evonode.json');
      mnsyncStatus = readCoreRpcFixture('mnsync-status.json');

      config.set('core.masternode.enable', true);

      rpcClient.masternode.withArgs('status').resolves({ result: masternodeStatus });
      rpcClient.mnsync.withArgs('status').resolves({ result: mnsyncStatus });
    });

    // The Core analysers judge PoSe state from one response and sync progress
    // from the other, so each must be stored under the name it is read by.
    it('should store each RPC response under its own sample', async () => {
      await collectSamples();

      expect(samples.getServiceInfo('core', 'masternodeStatus')).to.deep.equal(masternodeStatus);
      expect(samples.getServiceInfo('core', 'masternodeSyncStatus')).to.deep.equal(mnsyncStatus);
    });

    it('should report a PoSe-banned masternode', async () => {
      masternodeStatus.state = 'POSE_BANNED';

      await collectSamples();

      const problems = analyseCoreFactory()(samples);

      expect(problems.map((problem) => problem.getDescription())).to.include(
        'The masternode has been banned due to failing Proof-of-Service checks.',
      );
    });

    it('should report an evonode whose Tenderdash node ID is not the registered one', async () => {
      config.set('platform.drive.tenderdash.node.id', 'f'.repeat(40));

      await collectSamples();

      expect(analyseConfig(samples).map((problem) => problem.getDescription())).to.include(
        'Platform Node ID doesn\'t match the one found in the ProReg transaction',
      );
    });

    // Tenderdash takes its ID from the key, not from the saved ID, and
    // `config set` can change the key alone
    it('should report an evonode whose node key is not the registered one, even when its saved node ID is', async () => {
      config.set('platform.drive.tenderdash.node.key', generateTenderdashNodeKey());
      config.set('platform.drive.tenderdash.node.id', masternodeStatus.dmnState.platformNodeID);

      await collectSamples();

      expect(analyseConfig(samples).map((problem) => problem.getDescription())).to.include(
        'Platform Node ID doesn\'t match the one found in the ProReg transaction',
      );
    });

    // The schema requires neither the key nor the ID, and a masternode is
    // never given a generated key, so a config can hold only the saved ID
    it('should diagnose an evonode whose config has a saved node ID but no key', async () => {
      config.set('platform.drive.tenderdash.node', { id: 'f'.repeat(40) });

      await collectSamples();

      expect(samples.getServiceInfo('drive_tenderdash', 'nodeIdFromKey')).to.be.undefined();
      expect(analyseConfig(samples).map((problem) => problem.getDescription())).to.include(
        'Platform Node ID doesn\'t match the one found in the ProReg transaction',
      );
    });

    // The key is redacted from the report, so the ID it gives is collected
    // instead. The ID is public: Tenderdash announces it to every peer.
    it('should collect the node ID derived from the node key', async () => {
      const nodeKey = generateTenderdashNodeKey();
      config.set('platform.drive.tenderdash.node.key', nodeKey);

      await collectSamples();

      expect(samples.getServiceInfo('drive_tenderdash', 'nodeIdFromKey'))
        .to.equal(deriveTenderdashNodeId(nodeKey));
    });
  });

  describe('gateway TLS private key permissions', () => {
    /**
     * @return {string}
     */
    function keyFilePath() {
      return homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl', 'private.key');
    }

    beforeEach(() => {
      getCertificate.resolves(new Certificate({
        id: 'certificate-id',
        common_name: EXTERNAL_IP,
        status: 'issued',
        created: toZeroSslDate(daysFromNow(-1)),
        expires: toZeroSslDate(daysFromNow(89)),
      }));
    });

    it('should collect the mode of the gateway TLS private key', async () => {
      fs.chmodSync(keyFilePath(), 0o644);

      await collectSamples();

      expect(samples.getServiceInfo('gateway', 'sslPrivateKeyMode')).to.equal(0o644);
    });

    it('should collect nothing when there is no private key', async () => {
      fs.rmSync(keyFilePath());

      await collectSamples();

      expect(samples.getServiceInfo('gateway', 'sslPrivateKeyMode')).to.be.undefined();
    });

    // Windows reports a writable file as 0666 whatever its ACL allows, so the mode
    // bits would make doctor report an exposed key that is not exposed
    it('should collect nothing on Windows, where file modes are not permissions', async () => {
      const platform = Object.getOwnPropertyDescriptor(process, 'platform');

      fs.chmodSync(keyFilePath(), 0o644);
      Object.defineProperty(process, 'platform', { ...platform, value: 'win32' });

      try {
        await collectSamples();
      } finally {
        Object.defineProperty(process, 'platform', platform);
      }

      expect(samples.getServiceInfo('gateway', 'sslPrivateKeyMode')).to.be.undefined();
    });
  });

  // The report is the artefact an operator sends to support, and the command
  // tells them it carries no secrets. Containers get their credentials through
  // the environment, so the inspect sample is where they would end up.
  describe('container samples in the archive', () => {
    const SERVICE = { name: 'drive_abci', title: 'Drive ABCI' };

    // Deliberately not a value from the config: the environment has to be
    // redacted in its own right, not only because the same value is also
    // scrubbed as a known config secret.
    const ENVIRONMENT_SECRET = 'environmentOnlyValue7Qx2';

    let archiveRoot;
    let previousTmpDir;
    let getServiceList;

    /**
     * @return {Promise<string>} path to the archive
     */
    async function collectAndArchive() {
      await collectSamples();
      await archiveSamples(samples, archiveRoot, config);

      const [archiveName] = fs.readdirSync(archiveRoot)
        .filter((name) => name.endsWith('.tar.gz'));

      return path.join(archiveRoot, archiveName);
    }

    /**
     * A tarball stores member contents verbatim, so the decompressed stream
     * holds every byte of every file in the archive, whatever its name.
     *
     * @param {string} archivePath
     * @return {string}
     */
    function readArchivedBytes(archivePath) {
      return zlib.gunzipSync(fs.readFileSync(archivePath)).toString('utf8');
    }

    beforeEach(function beforeEach() {
      archiveRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-report-test-'));

      // The archive is staged under the system temp directory before it is packed
      previousTmpDir = process.env.TMPDIR;
      process.env.TMPDIR = archiveRoot;

      getServiceList = () => [SERVICE];

      this.sinon.stub(si, 'dockerContainerStats').resolves([]);

      collectSamplesTask = collectSamplesTaskFactory(
        dockerCompose,
        this.sinon.stub().returns(rpcClient),
        this.sinon.stub().resolves('127.0.0.1'),
        this.sinon.stub().returns({ request: this.sinon.stub().resolves({}) }),
        getServiceList,
        this.sinon.stub().resolves({}),
        homeDir,
        validateZeroSslCertificateFactory(homeDir, getCertificate),
        this.sinon.stub().resolves({}),
        checkGatewayCertificateFactory(homeDir),
        new RenewalRecordRepository(homeDir),
      );

      getCertificate.resolves(new Certificate({
        id: 'certificate-id',
        common_name: EXTERNAL_IP,
        status: 'issued',
        created: toZeroSslDate(daysFromNow(-1)),
        expires: toZeroSslDate(daysFromNow(89)),
      }));
    });

    afterEach(() => {
      if (previousTmpDir === undefined) {
        delete process.env.TMPDIR;
      } else {
        process.env.TMPDIR = previousTmpDir;
      }

      fs.rmSync(archiveRoot, { recursive: true, force: true });
    });

    it('should not archive the values of container environment variables', async () => {
      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Config: {
          Image: 'dashpay/drive:test',
          Env: [
            `CORE_CONSENSUS_JSON_RPC_PASSWORD=${ENVIRONMENT_SECRET}`,
            'NETWORK=testnet',
            'PROMETHEUS_BIND_ADDRESS=',
          ],
        },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });

      const archived = readArchivedBytes(await collectAndArchive());

      expect(archived).to.not.contain(ENVIRONMENT_SECRET);

      // Every value goes, not only the ones whose name looks sensitive: a
      // variable added later is redacted without anyone having to list it.
      expect(archived).to.not.contain('NETWORK=testnet');

      // Which variables a container was started with still tells a container
      // created before a variable existed apart from one created after.
      expect(archived).to.contain('"CORE_CONSENSUS_JSON_RPC_PASSWORD=<redacted>"');
      expect(archived).to.contain('"NETWORK=<redacted>"');
      expect(archived).to.contain('"PROMETHEUS_BIND_ADDRESS="');
    });

    it('should keep what the container analyser reads from an archived inspect', async () => {
      const state = {
        Status: 'restarting',
        Running: true,
        Restarting: true,
        OOMKilled: false,
        ExitCode: 137,
        Error: 'container exited unexpectedly',
      };

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Config: {
          Image: 'dashpay/drive:test',
          Env: [`CORE_CONSENSUS_JSON_RPC_PASSWORD=${ENVIRONMENT_SECRET}`],
        },
        State: state,
      });

      const restored = await unarchiveSamplesFactory(getServiceList)(await collectAndArchive());

      const inspect = restored.getServiceInfo(SERVICE.name, 'dockerInspect');

      expect(inspect.State).to.deep.equal(state);
      expect(inspect.Config.Image).to.equal('dashpay/drive:test');

      const problems = analyseServiceContainersFactory(getServiceList)(restored)
        .map((problem) => problem.getDescription());

      expect(problems).to.include(
        'Service Drive ABCI failed with an error code 137\nand message: container exited unexpectedly.',
      );
    });

    // Collected text has the operator's home directory rewritten as it is
    // taken in. A secret that starts with that directory must already be
    // masked by then, or what is left of it no longer reads as the secret.
    it('should mask a secret that holds the home directory before the directory is rewritten', async () => {
      const { homePath } = getOperatorIdentity();
      const secret = `${homePath}/pin7`;

      config.set('core.rpc.users.dapi.password', secret);

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Config: { Image: 'dashpay/dapi:test', Env: [] },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });
      dockerCompose.logs.resolves({ out: `auth with ${secret} rejected\n`, err: '' });

      const archived = readArchivedBytes(await collectAndArchive());

      expect(archived).to.not.contain('/pin7');
      expect(archived).to.contain(`auth with ${'*'.repeat(secret.length)} rejected`);
    });

    // A key passed to Core on its command line is in the config and in the
    // command the container was started with, and Core may print it as well
    it('should not archive a key passed to Core on its command line', async () => {
      const blsKey = '4a'.repeat(32);

      config.set('core.docker.commandArgs', [`-masternodeblsprivkey=${blsKey}`, '-printtoconsole']);

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Path: 'dashd',
        Args: [`-masternodeblsprivkey=${blsKey}`, '-printtoconsole'],
        Config: {
          Image: 'dashpay/dashd:test',
          Env: [],
          Cmd: ['dashd', `-masternodeblsprivkey=${blsKey}`, '-printtoconsole'],
          Entrypoint: ['/docker-entrypoint.sh', '--token=entrypointToken7Qx'],
        },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });
      dockerCompose.logs.resolves({
        out: `Command-line arg: masternodeblsprivkey="${blsKey}"\n`,
        err: '',
      });

      const archivePath = await collectAndArchive();
      const archived = readArchivedBytes(archivePath);

      expect(archived).to.not.contain(blsKey);

      const restored = await unarchiveSamplesFactory(getServiceList)(archivePath);
      const inspect = restored.getServiceInfo(SERVICE.name, 'dockerInspect');

      expect(inspect.Path).to.equal('dashd');
      expect(inspect.Args).to.equal('<redacted>');
      expect(inspect.Config.Cmd).to.equal('<redacted>');
      expect(inspect.Config.Entrypoint).to.equal('<redacted>');
      expect(archived).to.not.contain('entrypointToken7Qx');
      expect(restored.getDashmateConfig().get('core.docker.commandArgs')).to.deep.equal([
        `-masternodeblsprivkey=${'*'.repeat(blsKey.length)}`,
        '-printtoconsole',
      ]);
      expect(restored.getServiceInfo(SERVICE.name, 'stdOut')).to.contain('Command-line arg: masternodeblsprivkey="');
    });

    // An image can bring credentials of its own, which the config does not
    // know, and a command line has too many ways of carrying one to mask it
    // argument by argument. No analyser reads it, so it is left out; only the
    // executable stays.
    [
      ['a shell command line', { Cmd: ['sh', '-c', 'exec app --password shellSecret7Qx'] }],
      ['a value after its option', { Cmd: ['app', '--token', 'separateSecret7Qx'] }],
      ['a value that starts with a hyphen', { Cmd: ['app', '--token', '-hyphenSecret7Qx'] }],
      ['a positional value after the script name', {
        Cmd: ['sh', '-c', 'exec app --password "$1"', 'sh', 'positionalSecret7Qx'],
      }],
      ['an entrypoint argument', { Entrypoint: ['/entry.sh', 'entrypointSecret7Qx'] }],
      ['a health check', {
        Healthcheck: { Test: ['CMD-SHELL', 'curl -u probe:healthSecret7Qx http://localhost/'], Interval: 1e9 },
      }],
    ].forEach(([description, commandConfig]) => {
      it(`should leave the container command out of the archive, with ${description}`, async () => {
        const [secret] = JSON.stringify(commandConfig).match(/[A-Za-z]+Secret7Qx/);

        dockerCompose.inspectService.resolves({
          Id: 'container-id',
          Path: 'app',
          Args: ['--token', secret],
          Config: {
            Image: 'example/service:test',
            Env: [],
            ...commandConfig,
          },
          State: { Status: 'running', Restarting: false, ExitCode: 0 },
        });

        const archivePath = await collectAndArchive();

        expect(readArchivedBytes(archivePath)).to.not.contain(secret);

        const inspect = (await unarchiveSamplesFactory(getServiceList)(archivePath))
          .getServiceInfo(SERVICE.name, 'dockerInspect');

        expect(inspect.Path).to.equal('app');
        expect(inspect.Args).to.equal('<redacted>');
        ['Cmd', 'Entrypoint'].filter((field) => field in commandConfig)
          .forEach((field) => expect(inspect.Config[field]).to.equal('<redacted>'));

        if (commandConfig.Healthcheck) {
          expect(inspect.Config.Healthcheck.Test).to.equal('<redacted>');
          expect(inspect.Config.Healthcheck.Interval).to.equal(1e9);
        }
      });
    });

    // A health check prints whatever its probe prints, which can be a value
    // the image brings and the config does not know. No analyser reads it, so
    // it goes with the probe command; what says how the check went stays.
    it('should leave health check output out of the archive and keep its results', async () => {
      const secret = 'probeOutputSecret7Qx';
      const healthLog = {
        Start: '2026-11-26T10:00:00.000000000Z',
        End: '2026-11-26T10:00:01.000000000Z',
        ExitCode: 1,
        Output: `curl: (22) auth as probe:${secret} refused\n`,
      };

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Config: { Image: 'example/service:test', Env: [] },
        State: {
          Status: 'running',
          Restarting: false,
          ExitCode: 0,
          Health: { Status: 'unhealthy', FailingStreak: 3, Log: [healthLog] },
        },
      });

      const archivePath = await collectAndArchive();

      expect(readArchivedBytes(archivePath)).to.not.contain(secret);

      const { Health: health } = (await unarchiveSamplesFactory(getServiceList)(archivePath))
        .getServiceInfo(SERVICE.name, 'dockerInspect').State;

      expect(health).to.deep.equal({
        Status: 'unhealthy',
        FailingStreak: 3,
        Log: [{ ...healthLog, Output: '<redacted>' }],
      });
    });

    it('should collect a container whose command has more arguments than a call can take', async () => {
      const command = Array.from({ length: 200_000 }, (_, index) => `-o${index}=v${index}`);

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Path: 'app',
        Args: command,
        Config: { Image: 'example/service:test', Env: [], Cmd: command },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });

      const inspect = (await unarchiveSamplesFactory(getServiceList)(await collectAndArchive()))
        .getServiceInfo(SERVICE.name, 'dockerInspect');

      expect(inspect.Args).to.equal('<redacted>');
    });

    // A service can print an option value on its own, apart from the option
    it('should scrub a log of an option value with spaces in it printed alone', async () => {
      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Path: 'app',
        Args: ['--token=container token 7Qx'],
        Config: { Image: 'example/service:test', Env: [], Cmd: ['--token=container token 7Qx'] },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });
      dockerCompose.logs.resolves({ out: 'using token container token 7Qx\n', err: '' });

      const archived = readArchivedBytes(await collectAndArchive());

      expect(archived).to.not.contain('container token 7Qx');
      expect(archived).to.contain('using token ');
    });

    // Docker runs the entrypoint with the command after it, so an option at
    // the end of one takes its value from the start of the other
    it('should not archive an option value split between the entrypoint and the command', async () => {
      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Path: 'app',
        Args: ['--token', 'splitAcrossToken7Qx'],
        Config: {
          Image: 'example/service:test',
          Env: [],
          Entrypoint: ['app', '--token'],
          Cmd: ['splitAcrossToken7Qx'],
        },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });

      const archivePath = await collectAndArchive();

      expect(readArchivedBytes(archivePath)).to.not.contain('splitAcrossToken7Qx');

      const inspect = (await unarchiveSamplesFactory(getServiceList)(archivePath))
        .getServiceInfo(SERVICE.name, 'dockerInspect');

      expect(inspect.Config.Cmd).to.equal('<redacted>');
    });

    // A container can print the command it was started with, and the
    // credentials an image passes itself are not in the config
    it('should not archive an option value a container prints in its own log', async () => {
      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Path: 'app',
        Args: ['--token=loggedToken7Qx'],
        Config: { Image: 'example/service:test', Env: [], Cmd: ['app', '--token=loggedToken7Qx'] },
        State: { Status: 'running', Restarting: false, ExitCode: 0 },
      });
      dockerCompose.logs.resolves({ out: 'starting app --token=loggedToken7Qx\n', err: '' });

      const archived = readArchivedBytes(await collectAndArchive());

      expect(archived).to.not.contain('loggedToken7Qx');
      expect(archived).to.contain('starting app --token=');
    });

    // Doctor decides on the facts of a renewal record - which provider, what
    // outcome, when - live and when it reads a report back. A secret spelled
    // like one of them must not change them; the record's free text is
    // still scrubbed.
    it('should keep the facts of a renewal record when a secret is spelled like one', async () => {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');
      config.set('core.rpc.users.dapi.password', 'letsencrypt');

      new RenewalRecordRepository(homeDir).write(config.getName(), RenewalRecord.fromObject({
        provider: 'letsencrypt',
        outcome: 'failed',
        code: 'PORT_80_UNREACHABLE',
        detail: 'challenge failed with letsencrypt in the way',
        attemptedAt: new Date().toISOString(),
      }));

      const archivePath = await collectAndArchive();

      const live = samples.getServiceInfo('gateway', 'certificateRenewal');
      const restored = (await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath))
        .getServiceInfo('gateway', 'certificateRenewal');

      [live, restored].forEach((sample) => {
        const record = RenewalRecord.fromObject(sample);

        expect(record.isFailed()).to.be.true();
        expect(record.appliesTo({ provider: 'letsencrypt' })).to.be.true();
        expect(sample.detail).to.equal(`challenge failed with ${'*'.repeat('letsencrypt'.length)} in the way`);
      });
    });

    /**
     * @param {Object} sinon
     * @param {Object} overrides
     * @param {Function} [overrides.validateLetsEncryptCertificate]
     * @param {Function} [overrides.checkGatewayCertificate]
     */
    function useCollector(sinon, {
      validateLetsEncryptCertificate = sinon.stub().resolves({}),
      checkGatewayCertificate = checkGatewayCertificateFactory(homeDir),
      tenderdashRequest = sinon.stub().resolves({}),
    }) {
      collectSamplesTask = collectSamplesTaskFactory(
        dockerCompose,
        sinon.stub().returns(rpcClient),
        sinon.stub().resolves('127.0.0.1'),
        sinon.stub().returns({ request: tenderdashRequest }),
        getServiceList,
        sinon.stub().resolves({}),
        homeDir,
        validateZeroSslCertificateFactory(homeDir, getCertificate),
        validateLetsEncryptCertificate,
        checkGatewayCertificate,
        new RenewalRecordRepository(homeDir),
      );
    }

    // Doctor decides on the masternode's state and on whether the node ID
    // registered for it is the one Tenderdash has, live and when it reads a
    // report back. A secret spelled like the state, or like part of the ID,
    // must not change either verdict; the rest of Core's answer is still
    // scrubbed.
    [
      ['with only a saved node ID', () => ({ id: deriveTenderdashNodeId(generateTenderdashNodeKey()) })],
      ['with a node key', () => {
        const key = generateTenderdashNodeKey();

        return { id: deriveTenderdashNodeId(key), key };
      }],
    ].forEach(([name, makeNode]) => {
      it(`should keep the masternode state and node IDs when a secret is spelled like them, ${name}`, async function it() {
        const node = makeNode();
        const masternodeStatus = readCoreRpcFixture('masternode-status-evonode.json');

        masternodeStatus.state = 'POSE_BANNED';
        masternodeStatus.status = 'Banned, see POSE_BANNED';
        masternodeStatus.dmnState.platformNodeID = node.id;

        config.set('core.masternode.enable', true);
        config.set('platform.drive.tenderdash.node', node);
        config.set('core.rpc.users.dapi.password', 'POSE_BANNED');
        config.set('core.rpc.users.tenderdash.password', node.id.slice(4, 16));

        rpcClient.masternode.withArgs('status').resolves({ result: masternodeStatus });

        const tenderdashRequest = this.sinon.stub().resolves({});
        tenderdashRequest.withArgs('status').resolves({ result: { node_info: { id: node.id } } });

        useCollector(this.sinon, { tenderdashRequest });

        const archivePath = await collectAndArchive();
        const restored = await unarchiveSamplesFactory(
          () => [SERVICE, { name: 'core' }, { name: 'drive_tenderdash' }],
        )(archivePath);

        [samples, restored].forEach((collected) => {
          const where = collected === samples ? 'live' : 'from the archive';
          const configProblems = analyseConfig(collected).map((problem) => problem.getDescription());

          expect(analyseCoreFactory()(collected).map((problem) => problem.getDescription()), where)
            .to.include('The masternode has been banned due to failing Proof-of-Service checks.');
          expect(configProblems, where)
            .to.not.include('Platform Node ID doesn\'t match the one found in the ProReg transaction');
          expect(configProblems, where).to.not.include(
            'Tenderdash is running with a node ID other than the one found in the ProReg transaction',
          );
        });

        expect(restored.getServiceInfo('core', 'masternodeStatus').status)
          .to.equal(`Banned, see ${'*'.repeat('POSE_BANNED'.length)}`);
      });
    });

    // Doctor chooses the problem to report by the SSL check's error code, live
    // and when it reads a report back, so a secret spelled like the code must
    // not change it. What comes with the code is display text, and is still
    // scrubbed.
    it('should keep the SSL error code when a secret is spelled like it', async function it() {
      const code = 'CERTIFICATE_IP_MISMATCH';
      const masked = '*'.repeat(code.length);

      config.set('platform.gateway.ssl.provider', 'letsencrypt');
      config.set('core.rpc.users.dapi.password', code);

      useCollector(this.sinon, {
        validateLetsEncryptCertificate: this.sinon.stub().resolves({
          error: code,
          data: { externalIp: EXTERNAL_IP, legoCertPath: `/srv/lego/${code}/cert.crt` },
        }),
      });

      const archivePath = await collectAndArchive();
      const restored = await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath);

      [samples, restored].forEach((collected) => {
        const ssl = collected.getServiceInfo('gateway', 'ssl');

        expect(ssl.error).to.equal(code);
        expect(ssl.data.legoCertPath).to.equal(`/srv/lego/${masked}/cert.crt`);
        expect(analyseConfig(collected).map((problem) => problem.getDescription()))
          .to.include(`Let's Encrypt certificate does not match external IP ${EXTERNAL_IP}.`);
      });
    });

    // Doctor decides on the installed certificate's status, reason codes,
    // dates and fingerprint. Its messages are display text with paths in
    // them, and are still scrubbed.
    it('should keep the facts of the installed certificate when a secret is spelled like one', async function it() {
      const validFrom = new Date('2026-11-01T00:00:00Z');
      const validTo = new Date('2026-12-01T00:00:00Z');
      const fingerprint256 = 'AB:CD:EF:01:23:45:67:89';

      config.set('core.rpc.users.dapi.password', 'CHECKS_PASSED');
      config.set('core.rpc.users.tenderdash.password', 'PROVIDER_MISMATCH');
      config.set('core.rpc.users.quorum_list.password', fingerprint256);

      useCollector(this.sinon, {
        checkGatewayCertificate: () => ({
          status: 'CHECKS_PASSED',
          reasons: [],
          warnings: [{
            code: 'PROVIDER_MISMATCH',
            message: 'Issued by letsencrypt, see /srv/PROVIDER_MISMATCH/notes',
          }],
          skipped: ['IDENTITY'],
          provider: 'zerossl',
          installed: { validFrom, validTo, fingerprint256 },
          expiresInDays: 5,
        }),
      });

      const archivePath = await collectAndArchive();
      const restored = (await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath))
        .getServiceInfo('gateway', 'installedCertificate');

      [samples.getServiceInfo('gateway', 'installedCertificate'), restored].forEach((installed) => {
        expect(installed).to.deep.equal({
          status: 'CHECKS_PASSED',
          reasons: [],
          warnings: [{
            code: 'PROVIDER_MISMATCH',
            message: `Issued by letsencrypt, see /srv/${'*'.repeat('PROVIDER_MISMATCH'.length)}/notes`,
          }],
          skipped: ['IDENTITY'],
          provider: 'zerossl',
          expiresInDays: 5,
          validTo: validTo.toUTCString(),
          validFrom: validFrom.toUTCString(),
          fingerprint256,
        });
      });
    });

    // A certificate not issued for this node's address cannot be reinstalled,
    // so the reason code decides whether the repair forces a new one
    it('should keep a reason code that decides the repair when a secret is spelled like it', async function it() {
      config.set('core.rpc.users.dapi.password', 'IP_MISMATCH');

      useCollector(this.sinon, {
        checkGatewayCertificate: () => ({
          status: 'INVALID',
          reasons: [{ code: 'IP_MISMATCH', message: "The installed certificate is not issued for this node's address" }],
          warnings: [],
          skipped: [],
          provider: 'zerossl',
          installed: {
            validFrom: new Date('2026-11-01T00:00:00Z'),
            validTo: new Date('2026-12-01T00:00:00Z'),
            fingerprint256: 'AB:CD:EF:01:23:45:67:89',
          },
          expiresInDays: 5,
        }),
      });

      const archivePath = await collectAndArchive();
      const restored = await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath);

      [samples, restored].forEach((collected) => {
        const repairs = analyseGatewayCertificateFactory()(collected)
          .filter((problem) => problem.getDescription().includes('not issued for this node'))
          .map((problem) => problem.getSolution());

        expect(repairs).to.have.lengthOf(1);
        expect(repairs[0]).to.contain('--force');
      });
    });

    // Doctor only advises loading the saved certificate when the one it
    // judged on disk is the one the gateway probe read, by fingerprint
    it('should keep the fingerprints doctor compares when a secret is part of one', async () => {
      const { cert, key } = createCertificateForTest({
        ip: EXTERNAL_IP, days: 30, serialNumber: '7A1B2C3D4E5F',
      });
      const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');

      fs.writeFileSync(path.join(sslDir, 'bundle.crt'), cert, 'utf8');
      fs.writeFileSync(path.join(sslDir, 'private.key'), key, 'utf8');

      const server = tls.createServer({ cert, key }, (socket) => socket.end());
      const liveSockets = [];
      server.on('secureConnection', (socket) => liveSockets.push(socket));

      await new Promise((resolve) => { server.listen(0, '127.0.0.1', resolve); });

      config.set('platform.gateway.listeners.dapiAndDrive.port', server.address().port);

      const { fingerprint256, serialNumber } = new crypto.X509Certificate(cert);

      config.set('core.rpc.users.dapi.password', fingerprint256.slice(0, 23));
      // Nothing decides on the serial number, so it is scrubbed like the
      // rest of what the gateway presents
      config.set('core.rpc.users.tenderdash.password', serialNumber);

      let archivePath;

      try {
        archivePath = await collectAndArchive();
      } finally {
        liveSockets.forEach((socket) => socket.destroy());
        await new Promise((resolve) => { server.close(resolve); });
      }

      const restored = await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath);

      [samples, restored].forEach((collected) => {
        const served = collected.getServiceInfo('gateway', 'servedCertificate');

        expect(collected.getServiceInfo('gateway', 'installedCertificate').fingerprint256)
          .to.equal(fingerprint256);
        expect(served.certificate.fingerprint256).to.equal(fingerprint256);
        expect(served.onDisk.fingerprint256).to.equal(fingerprint256);
      });

      expect(samples.getServiceInfo('gateway', 'servedCertificate').certificate.serialNumber)
        .to.equal(serialNumber);
      expect(restored.getServiceInfo('gateway', 'servedCertificate').certificate.serialNumber)
        .to.equal('*'.repeat(serialNumber.length));
    });

    // The provider and the code of a renewal record are kept as they are,
    // because doctor decides on them. The file can be edited by hand, though,
    // so a value dashmate does not know is any text at all, and the report
    // carries a label in its place. The record on disk is still read as it is.
    it('should label a renewal provider and code it does not know as unknown', async () => {
      const handWritten = 'handWrittenValue7Qx';

      config.set('core.rpc.users.dapi.password', handWritten);

      const repository = new RenewalRecordRepository(homeDir);

      repository.write(config.getName(), RenewalRecord.fromObject({
        provider: `${handWritten}-provider`,
        outcome: 'failed',
        code: `${handWritten}-code`,
        detail: 'challenge failed',
        attemptedAt: new Date().toISOString(),
      }));

      const archivePath = await collectAndArchive();

      expect(readArchivedBytes(archivePath)).to.not.contain(handWritten);

      const live = samples.getServiceInfo('gateway', 'certificateRenewal');
      const restored = (await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath))
        .getServiceInfo('gateway', 'certificateRenewal');

      [live, restored].forEach((sample) => {
        expect(sample.provider).to.equal('unknown');
        expect(sample.code).to.equal('UNKNOWN');
        expect(RenewalRecord.fromObject(sample).isFailed()).to.be.true();
      });

      const onDisk = repository.read(config.getName()).record;

      expect(onDisk.getProvider()).to.equal(`${handWritten}-provider`);
      expect(onDisk.getCode()).to.equal(`${handWritten}-code`);
    });

    // Nothing doctor analyses reads the free text an operator writes in the
    // config, and that text can quote secrets and links in any form, such as
    // a build argument's URL in full with a token in its query. The report
    // carries only whether it was set.
    const buildUrl = "https://builder:buildPassword7Qx@builds.example.com/team's/drive.tar?sig=signedQueryToken7Qx&expires=99";

    [
      [
        'an authenticated URL with an apostrophe in its path, copied from a build argument',
        `drive built from ${buildUrl} for testing`,
        { PRIVATE_BUILD_URL: buildUrl },
        ['buildPassword7Qx', 'signedQueryToken7Qx', "team's/drive.tar", 'builds.example.com'],
      ],
      [
        'a URL with credentials and a query token',
        'mirror https://mirrorUser:mirrorPassword7Qx@mirror.example.net/private/core.tar?token=mirrorQueryToken7Qx for builds',
        null,
        ['mirrorUser', 'mirrorPassword7Qx', 'mirrorQueryToken7Qx', 'mirror.example.net/private/core.tar'],
      ],
      [
        'a plain sentence',
        'rack 7 in Frankfurt, run by the Quokka team',
        null,
        ['rack 7 in Frankfurt', 'Quokka team'],
      ],
    ].forEach(([kind, description, buildArgs, fragments]) => {
      it(`should leave the description out of the archive when it holds ${kind}`, async () => {
        if (buildArgs) {
          config.set('platform.drive.abci.docker.build.buildArgs', buildArgs);
        }

        config.set('description', description);

        const archivePath = await collectAndArchive();

        const archived = readArchivedBytes(archivePath);

        [description, ...fragments].forEach((fragment) => expect(archived).to.not.contain(fragment));

        const restoredConfig = (await unarchiveSamplesFactory(getServiceList)(archivePath))
          .getDashmateConfig();

        [samples.getDashmateConfig(), restoredConfig].forEach((collectedConfig) => {
          expect(collectedConfig.get('description')).to.equal('<redacted>');
        });
      });
    });

    // The Let's Encrypt check reports the contact address with the rest of
    // what it looked at, so it is left out there too. The config copy says
    // whether it is set, and its marker stays as it is even when a secret is
    // spelled like it.
    it('should leave the contact address out of the archive', async function it() {
      const email = 'quokka.ops+frankfurt@example.org';

      config.set('platform.gateway.ssl.provider', 'letsencrypt');
      config.set('platform.gateway.ssl.providerConfigs.letsencrypt.email', email);
      config.set('core.rpc.users.dapi.password', 'redacted');

      useCollector(this.sinon, {
        validateLetsEncryptCertificate: validateLetsEncryptCertificateFactory(homeDir),
      });

      const archivePath = await collectAndArchive();

      const archived = readArchivedBytes(archivePath);

      expect(archived).to.not.contain('quokka.ops');
      expect(archived).to.not.contain('frankfurt@example.org');

      const restored = await unarchiveSamplesFactory(() => [SERVICE, { name: 'gateway' }])(archivePath);

      [samples, restored].forEach((collected) => {
        expect(collected.getDashmateConfig().get('platform.gateway.ssl.providerConfigs.letsencrypt.email'))
          .to.equal('<redacted>');
        expect(collected.getServiceInfo('gateway', 'ssl').data).to.not.have.property('email');
      });
    });

    // A password can be spelled like a value the config schema requires
    // elsewhere, such as the environment name. Doctor validates the config copy
    // when it reads a report back, so the copy must keep that value while the
    // password itself stays hidden.
    it('should read a report back when a password equals another config value', async () => {
      config.set('environment', 'production');
      config.set('core.rpc.users.dapi.password', 'production');

      dockerCompose.inspectService.resolves({
        Id: 'container-id',
        Config: { Image: 'dashpay/drive:test', Env: [] },
        State: {
          Restarting: true,
          ExitCode: 1,
          Error: 'container exited unexpectedly',
        },
      });

      const restored = await unarchiveSamplesFactory(getServiceList)(await collectAndArchive());

      const restoredConfig = restored.getDashmateConfig();

      expect(restoredConfig.get('environment')).to.equal('production');
      expect(restoredConfig.get('core.rpc.users.dapi.password')).to.equal('*'.repeat('production'.length));

      const problems = analyseServiceContainersFactory(getServiceList)(restored)
        .map((problem) => problem.getDescription());

      expect(problems).to.include(
        'Service Drive ABCI failed with an error code 1\nand message: container exited unexpectedly.',
      );
      expect(() => analyseConfig(restored)).to.not.throw();
    });
  });
});

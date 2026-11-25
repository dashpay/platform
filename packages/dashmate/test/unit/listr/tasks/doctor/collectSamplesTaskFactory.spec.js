import fs from 'fs';
import os from 'os';
import path from 'path';
import tls from 'node:tls';
import { Listr } from 'listr2';
import getBaseConfigFactory from '../../../../../configs/defaults/getBaseConfigFactory.js';
import getLocalConfigFactory from '../../../../../configs/defaults/getLocalConfigFactory.js';
import getMainnetConfigFactory from '../../../../../configs/defaults/getMainnetConfigFactory.js';
import getTestnetConfigFactory from '../../../../../configs/defaults/getTestnetConfigFactory.js';
import DefaultConfigs from '../../../../../src/config/DefaultConfigs.js';
import Config from '../../../../../src/config/Config.js';
import ConfigFile from '../../../../../src/config/configFile/ConfigFile.js';
import HomeDir from '../../../../../src/config/HomeDir.js';
import createCertificateForTest from '../../../../../src/test/createCertificateForTest.js';
import analyseActivationFactory from '../../../../../src/doctor/analyse/analyseActivationFactory.js';
import analyseConfigFactory from '../../../../../src/doctor/analyse/analyseConfigFactory.js';
import { SEVERITY } from '../../../../../src/doctor/Prescription.js';
import Samples from '../../../../../src/doctor/Samples.js';
import collectSamplesTaskFactory from '../../../../../src/listr/tasks/doctor/collectSamplesTaskFactory.js';
import Certificate from '../../../../../src/ssl/zerossl/Certificate.js';
import checkGatewayCertificateFactory from '../../../../../src/ssl/checkGatewayCertificateFactory.js';
import validateZeroSslCertificateFactory, { ERRORS as ZEROSSL_ERRORS } from '../../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import providers from '../../../../../src/status/providers.js';
import RenewalRecordRepository from '../../../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import { DASHMATE_HELPER_DOCKER_IMAGE, DASHMATE_VERSION } from '../../../../../src/constants.js';
import { ACTIVATION_STATE } from '../../../../../src/update/probeActivationState.js';

const EXTERNAL_IP = '198.51.100.7';

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
  let configFile;
  let defaultConfigs;

  // No migration is keyed above this dashmate, so it produces its own version as the format
  const getConfigFileMigrations = () => ({ '0.1.0': (configFileData) => configFileData });

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

  beforeEach(function beforeEach() {
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
    fs.writeFileSync(path.join(sslDir, 'csr.pem'), 'csr', 'utf8');
    fs.writeFileSync(path.join(sslDir, 'private.key'), 'private key', 'utf8');
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
      getContainersList: this.sinon.stub().resolves([]),
      isNodeRunning: this.sinon.stub().resolves(false),
    };

    const getBaseConfig = getBaseConfigFactory();

    defaultConfigs = new DefaultConfigs([
      getBaseConfig,
      getLocalConfigFactory(getBaseConfig),
      getTestnetConfigFactory(homeDir, getBaseConfig),
      getMainnetConfigFactory(homeDir, getBaseConfig),
    ]);

    // Holds only the base config, which is never a node, unless a test adds nodes to it
    configFile = new ConfigFile([getBaseConfig()], DASHMATE_VERSION, 'abcdef12', null, null);

    // The config file on disk is migrated to the format this dashmate produces
    fs.writeFileSync(
      homeDir.joinPath('config.json'),
      JSON.stringify({ configFormatVersion: DASHMATE_VERSION }),
      'utf8',
    );

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
      configFile,
      getConfigFileMigrations,
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
        configFile,
        getConfigFileMigrations,
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

  describe('node activation', () => {
    let mainnet;

    /**
     * @param {Config} node - the node set up on this host, next to the stock configs every host
     *   carries
     */
    function hostConfigs(node) {
      const localNode = new Config('local_1', defaultConfigs.get('local').getStoredOptions());
      localNode.set('externalIp', EXTERNAL_IP);

      configFile.setConfigs([
        defaultConfigs.get('base'),
        defaultConfigs.get('local'),
        localNode,
        defaultConfigs.get('testnet'),
        node,
      ]);
    }

    /**
     * Only the node set up on mainnet has containers; the stock testnet config was never started.
     *
     * @param {string} image
     */
    function mainnetHelperRuns(image) {
      dockerCompose.getContainersList.callsFake(async (probed) => (
        probed.getName() === mainnet.getName() ? [{ State: 'running', Image: image }] : []
      ));
    }

    /**
     * @return {Object}
     */
    function sampledMainnet() {
      return samples.getActivation().configs.find((sampled) => sampled.name === mainnet.getName());
    }

    beforeEach(() => {
      // The doctored config's own certificate checks are not what these tests are about
      config.set('platform.enable', false);

      mainnet = defaultConfigs.get('mainnet');
      mainnet.set('externalIp', EXTERNAL_IP);
      mainnet.set('core.masternode.enable', true);

      hostConfigs(mainnet);
    });

    it('should report a node whose helper still runs the image of the previous dashmate', async () => {
      mainnetHelperRuns('dashpay/dashmate-helper:0.0.1');

      await collectSamples();

      const problems = analyseActivationFactory()(samples);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getSeverity()).to.equal(SEVERITY.MEDIUM);
      expect(problems[0].getSolution()).to.include('dashmate restart --config mainnet --safe');
    });

    it('should report nothing for a node that runs the installed dashmate', async () => {
      mainnetHelperRuns(DASHMATE_HELPER_DOCKER_IMAGE);

      await collectSamples();

      expect(sampledMainnet().state).to.equal(ACTIVATION_STATE.ACTIVATED);
      expect(analyseActivationFactory()(samples)).to.be.empty();
    });

    // Local presets are run and restarted through their group, and base is only a template.
    it('should probe every config except base and the local network ones', async () => {
      mainnetHelperRuns(DASHMATE_HELPER_DOCKER_IMAGE);

      await collectSamples();

      expect(dockerCompose.getContainersList.getCalls().map((call) => call.args[0].getName()))
        .to.deep.equal(['testnet', 'mainnet']);
      expect(samples.getActivation().configs.map((sampled) => sampled.name))
        .to.deep.equal(['testnet', 'mainnet']);
    });

    // A config nobody set up has no containers, and is no more a problem than a stopped node.
    it('should report nothing for a stock config that was never started', async () => {
      mainnetHelperRuns(DASHMATE_HELPER_DOCKER_IMAGE);

      await collectSamples();

      const testnet = samples.getActivation().configs.find((sampled) => sampled.name === 'testnet');

      expect(testnet.nodeRunning).to.be.false();
      expect(analyseActivationFactory()(samples)).to.be.empty();
    });

    // The helper alone being down says nothing about whether the rest of the node still runs
    // what the previous dashmate started.
    it('should record whether the node runs when its helper does not', async () => {
      dockerCompose.isNodeRunning.callsFake(async (probed) => probed === mainnet);

      await collectSamples();

      expect(sampledMainnet().nodeRunning).to.be.true();

      const problems = analyseActivationFactory()(samples);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0].getSeverity()).to.equal(SEVERITY.LOW);
      expect(problems[0].getDescription()).to.include('mainnet');
    });

    it('should record that it could not tell whether a node without a helper runs', async () => {
      dockerCompose.isNodeRunning.callsFake(async (probed) => {
        if (probed === mainnet) {
          throw new Error('Cannot connect to the Docker daemon');
        }

        return false;
      });

      await collectSamples();

      expect(sampledMainnet().nodeRunning).to.be.null();
      expect(analyseActivationFactory()(samples)).to.have.lengthOf(1);
    });

    it('should report nothing for a stopped node', async () => {
      await collectSamples();

      expect(sampledMainnet().nodeRunning).to.be.false();
      expect(analyseActivationFactory()(samples)).to.be.empty();
    });

    // Docker being unavailable is already one high severity problem; it must not also become an
    // unknown for every node on the host.
    it('should not probe the nodes when docker is not available', async () => {
      dockerCompose.throwErrorIfNotInstalled.rejects(new Error('Docker is not installed'));

      await collectSamples();

      expect(dockerCompose.getContainersList).to.not.have.been.called();
      expect(samples.getActivation().error).to.be.a('string');
      expect(analyseActivationFactory()(samples)).to.be.empty();
    });

    it('should not fail the collection when the configs cannot be read', async () => {
      configFile.getAllConfigs = () => {
        throw new Error('broken config file');
      };

      await collectSamples();

      expect(samples.getActivation().error).to.include('broken config file');
    });

    // A docker error can name a path under the operator's home directory, and the report is
    // handed to whoever is helping. The config name stays as it is: masking it would break the
    // command the operator is told to run.
    it('should mask the operator\'s name in probe messages but not in config names', async () => {
      mainnet = new Config(operator, mainnet.getStoredOptions());
      hostConfigs(mainnet);

      dockerCompose.getContainersList
        .rejects(new Error(`Cannot connect to /home/${operator}/.docker/run/docker.sock`));

      await collectSamples();

      const sampled = sampledMainnet();

      expect(sampled.name).to.equal(operator);
      expect(sampled.message).to.not.include(`/home/${operator}/`);
      expect(sampled.helper.message).to.not.include(`/home/${operator}/`);
    });
  });
});

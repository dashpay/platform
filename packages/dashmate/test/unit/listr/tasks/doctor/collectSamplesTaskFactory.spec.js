import fs from 'fs';
import os from 'os';
import path from 'path';
import tls from 'node:tls';
import { Listr } from 'listr2';
import getBaseConfigFactory from '../../../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../../../src/config/HomeDir.js';
import createCertificateForTest from '../../../../../src/test/createCertificateForTest.js';
import analyseConfigFactory from '../../../../../src/doctor/analyse/analyseConfigFactory.js';
import { SEVERITY } from '../../../../../src/doctor/Prescription.js';
import Samples from '../../../../../src/doctor/Samples.js';
import collectSamplesTaskFactory from '../../../../../src/listr/tasks/doctor/collectSamplesTaskFactory.js';
import Certificate from '../../../../../src/ssl/zerossl/Certificate.js';
import checkGatewayCertificateFactory from '../../../../../src/ssl/checkGatewayCertificateFactory.js';
import validateZeroSslCertificateFactory, { ERRORS as ZEROSSL_ERRORS } from '../../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import providers from '../../../../../src/status/providers.js';
import RenewalRecordRepository from '../../../../../src/ssl/renewalRecord/RenewalRecordRepository.js';

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
});

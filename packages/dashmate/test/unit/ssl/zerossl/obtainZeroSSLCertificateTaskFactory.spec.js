import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import forge from 'node-forge';
import obtainZeroSSLCertificateTaskFactory from '../../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import validateZeroSslCertificateFactory, { ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import Certificate from '../../../../src/ssl/zerossl/Certificate.js';
import generateCertificateRequest from '../../../../src/ssl/zerossl/generateCsr.js';
import getEnquirerMock from '../../../../src/test/mock/getEnquirerMock.js';
import loadCertificateRequest from '../../../../src/ssl/zerossl/loadCertificateRequest.js';
import {
  createCertificateRequest,
  issueCertificate,
  issueCertificateForKey,
} from '../../../../src/test/certificateFixtures.js';

describe('obtainZeroSSLCertificateTaskFactory', () => {
  let config;
  let verificationServer;
  let validateZeroSslCertificate;
  let obtainZeroSSLCertificateTask;
  let pair;

  /**
   * Put a certificate pair the gateway can serve where the task finds it
   *
   * @param {string} bundleFilePath
   * @param {string} privateKeyFilePath
   */
  function writeCertificatePair(bundleFilePath, privateKeyFilePath) {
    fs.mkdirSync(path.dirname(bundleFilePath), { recursive: true });
    fs.writeFileSync(bundleFilePath, pair.pem);
    fs.writeFileSync(privateKeyFilePath, pair.keyPem, { mode: 0o600 });
  }

  before(function before() {
    // RSA key generation is slow in pure JS.
    this.timeout(30000);
    pair = issueCertificate({ ip: '127.0.0.1' });
  });

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('local'),
    };
    config.get
      .withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
      .returns('test-api-key');
    config.get.withArgs('externalIp').returns('1.2.3.4');

    verificationServer = {
      setup: this.sinon.stub().resolves(),
      start: this.sinon.stub().resolves(),
      stop: this.sinon.stub().resolves(),
      destroy: this.sinon.stub().resolves(),
      waitForServerIsResponding: this.sinon.stub().resolves(true),
    };

    // The first pipeline task calls validateZeroSslCertificate. Rejecting it
    // simulates a mid-pipeline failure (e.g. the ZeroSSL API going down), which
    // is the path that previously left the verification server bound to port 80.
    validateZeroSslCertificate = this.sinon.stub().rejects(new Error('ZeroSSL API unavailable'));

    obtainZeroSSLCertificateTask = obtainZeroSSLCertificateTaskFactory(
      this.sinon.stub(), // generateCsr
      this.sinon.stub(), // generateKeyPair
      this.sinon.stub(), // createZeroSSLCertificate
      this.sinon.stub(), // verifyDomain
      this.sinon.stub(), // downloadCertificate
      this.sinon.stub(), // getCertificate
      this.sinon.stub(), // listCertificates
      this.sinon.stub(), // saveCertificateTask
      verificationServer,
      { joinPath: this.sinon.stub().callsFake((...parts) => path.join('/tmp', ...parts)) }, // homeDir
      validateZeroSslCertificate,
    );
  });

  it('should stop and destroy the verification server when the pipeline fails', async function it() {
    const tasks = obtainZeroSSLCertificateTask(config, {
      onCertificateCreated: this.sinon.stub(),
    });

    let thrownError;
    try {
      await tasks.run({ expirationDays: 30 });
    } catch (e) {
      thrownError = e;
    }

    // The original pipeline error must still propagate — cleanup must not mask it.
    expect(thrownError).to.be.an('error');
    expect(thrownError.message).to.equal('ZeroSSL API unavailable');

    // ...and the verification server must be torn down so port 80 is freed,
    // even though the "Stop verification server" task never ran.
    expect(verificationServer.stop).to.have.been.called();
    expect(verificationServer.destroy).to.have.been.called();
  });

  it('should initialize configuration before forced regeneration', async function it() {
    const generateKeyPair = this.sinon.stub().resolves({ privateKey: 'private-key' });
    const generateCsr = this.sinon.stub().resolves('certificate-request');
    const sentinel = new Error('stop after certificate request');
    const createCertificate = this.sinon.stub().rejects(sentinel);
    const validate = this.sinon.stub().rejects(new Error('validation must be skipped'));
    const homeDir = {
      joinPath: this.sinon.stub().callsFake((...parts) => path.join('/tmp', ...parts)),
    };
    const task = obtainZeroSSLCertificateTaskFactory(
      generateCsr,
      generateKeyPair,
      createCertificate,
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      verificationServer,
      homeDir,
      validate,
    )(config, {
      onCertificateCreated: this.sinon.stub(),
    });

    await expect(task.run({
      expirationDays: 30,
      force: true,
      isCsrFilePresent: true,
      isPrivateKeyFilePresent: true,
      isBundleFilePresent: true,
      certificate: { id: 'old-certificate' },
    })).to.be.rejectedWith(sentinel);

    expect(validate).to.not.have.been.called();
    expect(generateKeyPair).to.have.been.calledOnce();
    expect(generateCsr).to.have.been.calledOnceWithExactly(
      { privateKey: 'private-key' },
      '1.2.3.4',
    );
    expect(createCertificate).to.have.been.calledOnceWithExactly(
      'certificate-request',
      '1.2.3.4',
      'test-api-key',
    );
  });

  it('should defer SSL persistence until command finalization', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      const configFile = new ConfigFile(
        [realConfig],
        '4.1.0',
        'abcdef12',
        realConfig.getName(),
        null,
      );
      const configFilePath = homeDir.joinPath('config.json');

      fs.writeFileSync(
        configFilePath,
        `${JSON.stringify(configFile.toObject(), undefined, 2)}\n`,
        'utf8',
      );

      const repository = new ConfigFileJsonRepository(
        (data) => data,
        homeDir,
        () => null,
      );
      const sslConfigDir = homeDir.joinPath('ssl');
      const validate = this.sinon.stub().resolves({
        error: ERRORS.CERTIFICATE_ID_IS_NOT_SET,
        data: {
          apiKey: 'api-key',
          bundleFilePath: homeDir.joinPath('bundle.crt'),
          certificate: null,
          csr: 'certificate-request',
          csrFilePath: homeDir.joinPath('certificate.csr'),
          externalIp: '127.0.0.1',
          isBundleFilePresent: true,
          isCsrFilePresent: true,
          isPrivateKeyFilePresent: true,
          privateKeyFilePath: homeDir.joinPath('private.key'),
          sslConfigDir,
        },
      });
      const createCertificate = this.sinon.stub().resolves({
        id: 'certificate-id-000000000000000000',
        status: 'issued',
      });
      writeCertificatePair(homeDir.joinPath('bundle.crt'), homeDir.joinPath('private.key'));
      const task = obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub(),
        this.sinon.stub(),
        createCertificate,
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validate,
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });

      await task.run({ expirationDays: 30 });

      expect(realConfig.isChanged()).to.be.true();
      expect(repository.read().getConfig(realConfig.getName())
        .get('platform.gateway.ssl.enabled')).to.be.false();
      expect(repository.read().getConfig(realConfig.getName())
        .get('platform.gateway.ssl.providerConfigs.zerossl.id')).to.equal(null);
    } finally {
      homeDir.remove();
    }
  });

  it('should not enable SSL when verification fails after creating a certificate', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      const configFile = new ConfigFile(
        [realConfig],
        '4.1.0',
        'abcdef12',
        realConfig.getName(),
        null,
      );
      const repository = new ConfigFileJsonRepository(
        (data) => data,
        homeDir,
        () => null,
      );
      repository.write(configFile);

      const validate = this.sinon.stub().resolves({
        error: ERRORS.CERTIFICATE_ID_IS_NOT_SET,
        data: {
          apiKey: 'api-key',
          bundleFilePath: homeDir.joinPath('bundle.crt'),
          certificate: null,
          csrFilePath: homeDir.joinPath('certificate.csr'),
          externalIp: '127.0.0.1',
          isBundleFilePresent: false,
          isCsrFilePresent: false,
          isPrivateKeyFilePresent: false,
          privateKeyFilePath: homeDir.joinPath('private.key'),
          sslConfigDir: homeDir.joinPath('ssl'),
        },
      });
      const verifyDomain = this.sinon.stub().rejects(new Error('verification failed'));
      const task = obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub().resolves('certificate-request'),
        this.sinon.stub().resolves({ privateKey: 'private-key' }),
        this.sinon.stub().resolves({
          id: 'certificate-id-000000000000000000',
          status: 'pending_validation',
          validation: {
            other_methods: {
              '127.0.0.1': {
                file_validation_url_http: 'http://127.0.0.1/verification',
                file_validation_content: ['verification'],
              },
            },
          },
        }),
        verifyDomain,
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validate,
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });

      await expect(task.run({ expirationDays: 30, noRetry: true }))
        .to.be.rejectedWith('verification failed');

      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.false();
      expect(realConfig.get('platform.gateway.ssl.providerConfigs.zerossl.id'))
        .to.equal('certificate-id-000000000000000000');
      expect(repository.read().getConfig(realConfig.getName())
        .get('platform.gateway.ssl.enabled')).to.be.false();
    } finally {
      homeDir.remove();
    }
  });

  it('should require a certificate-created persistence callback', () => {
    expect(() => obtainZeroSSLCertificateTask(config))
      .to.throw('onCertificateCreated callback is required');
  });

  it('should activate a resumed issued certificate', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      realConfig.set('platform.gateway.ssl.provider', 'self-signed');
      writeCertificatePair(homeDir.joinPath('bundle.crt'), homeDir.joinPath('private.key'));
      const validate = this.sinon.stub().resolves({
        data: {
          apiKey: 'api-key',
          bundleFilePath: homeDir.joinPath('bundle.crt'),
          certificate: {
            expires: new Date('2026-10-01T00:00:00.000Z'),
            status: 'issued',
          },
          csrFilePath: homeDir.joinPath('certificate.csr'),
          externalIp: '127.0.0.1',
          isBundleFilePresent: true,
          isCsrFilePresent: true,
          isPrivateKeyFilePresent: true,
          privateKeyFilePath: homeDir.joinPath('private.key'),
          sslConfigDir: homeDir.joinPath('ssl'),
        },
      });
      const task = obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validate,
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });

      await task.run({ expirationDays: 30 });

      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();
      expect(realConfig.get('platform.gateway.ssl.provider')).to.equal('zerossl');
    } finally {
      homeDir.remove();
    }
  });

  // The files are checked once more before TLS is enabled, whatever the
  // validation said about them: TLS enabled over a bundle cut short would
  // serve nothing.
  it('should not enable TLS until the certificate files on disk can be served', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      realConfig.set('platform.gateway.ssl.provider', 'zerossl');
      const sslConfigDir = homeDir.joinPath('ssl');
      const bundleFilePath = path.join(sslConfigDir, 'bundle.crt');
      const privateKeyFilePath = path.join(sslConfigDir, 'private.key');
      writeCertificatePair(bundleFilePath, privateKeyFilePath);
      fs.writeFileSync(bundleFilePath, pair.pem.slice(0, pair.pem.length / 2));

      const validate = this.sinon.stub().resolves({
        data: {
          apiKey: 'api-key',
          bundleFilePath,
          certificate: {
            expires: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000),
            status: 'issued',
          },
          csrFilePath: path.join(sslConfigDir, 'csr.pem'),
          externalIp: '127.0.0.1',
          isBundleFilePresent: true,
          isCsrFilePresent: true,
          isPrivateKeyFilePresent: true,
          privateKeyFilePath,
          sslConfigDir,
        },
      });
      const buildTask = () => obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validate,
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });

      await expect(buildTask().run({ expirationDays: 30 }))
        .to.be.rejectedWith('cannot be served');
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.false();

      fs.writeFileSync(bundleFilePath, pair.pem);

      await buildTask().run({ expirationDays: 30 });

      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();
    } finally {
      homeDir.remove();
    }
  });

  it('should create the ZeroSSL private key with mode 0600', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      const privateKeyFilePath = homeDir.joinPath('ssl', 'private.key');
      // The bundle of the issued certificate is already there.
      fs.mkdirSync(homeDir.joinPath('ssl'), { recursive: true });
      fs.writeFileSync(homeDir.joinPath('ssl', 'bundle.crt'), pair.pem);
      const validate = this.sinon.stub().resolves({
        error: ERRORS.CERTIFICATE_ID_IS_NOT_SET,
        data: {
          apiKey: 'api-key',
          bundleFilePath: homeDir.joinPath('ssl', 'bundle.crt'),
          certificate: null,
          csrFilePath: homeDir.joinPath('ssl', 'certificate.csr'),
          externalIp: '127.0.0.1',
          isBundleFilePresent: true,
          isCsrFilePresent: false,
          isPrivateKeyFilePresent: false,
          privateKeyFilePath,
          sslConfigDir: homeDir.joinPath('ssl'),
        },
      });
      const task = obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub().resolves('certificate-request'),
        this.sinon.stub().resolves({ privateKey: pair.keyPem }),
        this.sinon.stub().resolves({
          id: 'certificate-id-000000000000000000',
          status: 'issued',
        }),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validate,
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });

      await task.run({ expirationDays: 30 });

      // eslint-disable-next-line no-bitwise
      expect(fs.statSync(privateKeyFilePath).mode & 0o777).to.equal(0o600);
    } finally {
      homeDir.remove();
    }
  });

  // ZeroSSL saves the certificate ID before verification, and a run can stop
  // after the key is written and before the bundle is written in full: out
  // of disk space, say. The certificate stays issued, and a plain obtain has
  // to install it again rather than ask for another one.
  describe('a certificate ZeroSSL already issued', () => {
    const CERTIFICATE_ID = 'c'.repeat(32);

    let homeDir;
    let realConfig;
    let sslConfigDir;
    let bundleFilePath;
    let privateKeyFilePath;
    let createCertificate;
    let downloadCertificate;
    let other;

    /**
     * ZeroSSL's own date format
     *
     * @param {Date} date
     * @return {string}
     */
    function toZeroSslDate(date) {
      const pad = (number) => String(number).padStart(2, '0');

      return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
        + ` ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
    }

    let generateKeyPair;
    let fingerprint;
    let zeroSslError;

    function buildTask() {
      const getCertificate = zeroSslError
        ? this.sinon.stub().rejects(zeroSslError)
        : this.sinon.stub().resolves(new Certificate({
          id: CERTIFICATE_ID,
          common_name: '1.2.3.4',
          status: 'issued',
          created: toZeroSslDate(new Date(Date.now() - 24 * 60 * 60 * 1000)),
          expires: toZeroSslDate(new Date(Date.now() + 60 * 24 * 60 * 60 * 1000)),
          fingerprint_sha1: fingerprint,
        }));

      return obtainZeroSSLCertificateTaskFactory(
        generateCertificateRequest,
        generateKeyPair,
        createCertificate,
        this.sinon.stub(),
        downloadCertificate,
        getCertificate,
        this.sinon.stub(),
        this.sinon.stub(),
        verificationServer,
        homeDir,
        validateZeroSslCertificateFactory(homeDir, getCertificate),
      )(realConfig, {
        onCertificateCreated: this.sinon.stub(),
      });
    }

    before(function before() {
      this.timeout(30000);
      other = issueCertificate({ ip: '1.2.3.4' });
    });

    beforeEach(function beforeEach() {
      homeDir = HomeDir.createTemp();
      realConfig = getBaseConfigFactory(homeDir)();
      realConfig.set('externalIp', '1.2.3.4');
      realConfig.set('platform.gateway.ssl.provider', 'zerossl');
      realConfig.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', 'k'.repeat(32));
      realConfig.set('platform.gateway.ssl.providerConfigs.zerossl.id', CERTIFICATE_ID);

      sslConfigDir = homeDir.joinPath(realConfig.getName(), 'platform', 'gateway', 'ssl');
      bundleFilePath = path.join(sslConfigDir, 'bundle.crt');
      privateKeyFilePath = path.join(sslConfigDir, 'private.key');
      writeCertificatePair(bundleFilePath, privateKeyFilePath);
      fs.writeFileSync(path.join(sslConfigDir, 'csr.pem'), 'certificate-request');
      fs.writeFileSync(bundleFilePath, pair.pem.slice(0, pair.pem.length / 2));

      createCertificate = this.sinon.stub().rejects(new Error('a new certificate was requested'));
      downloadCertificate = this.sinon.stub().resolves(pair.pem);
      generateKeyPair = this.sinon.stub().rejects(new Error('a new key was generated'));
      fingerprint = null;
      zeroSslError = null;
    });

    afterEach(() => {
      homeDir.remove();
    });

    it('should download it again over a truncated bundle, without creating one', async function it() {
      // Each download attempt waits five seconds first.
      this.timeout(30000);

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(createCertificate).to.not.have.been.called();
      expect(downloadCertificate).to.have.been.calledOnceWith(CERTIFICATE_ID);
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(pair.pem);
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();
    });

    // Renewal requests the next certificate with csr.pem, and only the key on
    // disk serves it.
    it('should write a missing certificate request for the key on disk', async function it() {
      this.timeout(30000);
      fs.rmSync(path.join(sslConfigDir, 'csr.pem'));

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(generateKeyPair).to.not.have.been.called();
      expect(fs.readFileSync(privateKeyFilePath, 'utf8')).to.equal(pair.keyPem);

      const request = forge.pki.certificationRequestFromPem(
        fs.readFileSync(path.join(sslConfigDir, 'csr.pem'), 'utf8'),
      );
      const asSpki = (pem) => crypto.createPublicKey(pem).export({ type: 'spki', format: 'pem' });

      expect(asSpki(forge.pki.publicKeyToPem(request.publicKey))).to.equal(asSpki(pair.keyPem));
    });

    // The gateway loads the key from private.key, whatever follows it.
    it('should write the request for the key, not a certificate after it', async function it() {
      this.timeout(30000);
      fs.writeFileSync(privateKeyFilePath, `${pair.keyPem}${other.pem}`);
      fs.rmSync(path.join(sslConfigDir, 'csr.pem'));

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      const request = forge.pki.certificationRequestFromPem(
        fs.readFileSync(path.join(sslConfigDir, 'csr.pem'), 'utf8'),
      );
      const asSpki = (pem) => crypto.createPublicKey(pem).export({ type: 'spki', format: 'pem' });

      expect(request.verify()).to.be.true();
      expect(asSpki(forge.pki.publicKeyToPem(request.publicKey))).to.equal(asSpki(pair.keyPem));
    });

    // Renewal sends csr.pem to ZeroSSL as it is: a request a full disk cut
    // short would fail every renewal of the certificate installed here.
    it('should write a new certificate request for the key on disk over one cut short', async function it() {
      this.timeout(30000);
      const csrFilePath = path.join(sslConfigDir, 'csr.pem');
      const request = await generateCertificateRequest({
        publicKey: forge.pki.publicKeyToPem(pair.keys.publicKey),
        privateKey: pair.keyPem,
      }, '1.2.3.4');
      fs.writeFileSync(csrFilePath, request.slice(0, request.length / 2));

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(createCertificate).to.not.have.been.called();
      expect(generateKeyPair).to.not.have.been.called();
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(pair.pem);

      const written = forge.pki.certificationRequestFromPem(fs.readFileSync(csrFilePath, 'utf8'));
      const asSpki = (pem) => crypto.createPublicKey(pem).export({ type: 'spki', format: 'pem' });

      expect(written.verify()).to.be.true();
      expect(asSpki(forge.pki.publicKeyToPem(written.publicKey))).to.equal(asSpki(pair.keyPem));
    });

    // A link where the request goes is replaced, not written through: one
    // that points at the key would lose the key the certificate was issued for.
    it('should replace a link where the certificate request goes rather than write through it', async function it() {
      this.timeout(30000);
      const csrFilePath = path.join(sslConfigDir, 'csr.pem');
      fs.rmSync(csrFilePath);
      fs.symlinkSync(privateKeyFilePath, csrFilePath);

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(fs.readFileSync(privateKeyFilePath, 'utf8')).to.equal(pair.keyPem);
      expect(fs.lstatSync(csrFilePath).isSymbolicLink()).to.be.false();
      expect(() => forge.pki.certificationRequestFromPem(fs.readFileSync(csrFilePath, 'utf8')))
        .to.not.throw();
      expect(fs.readdirSync(sslConfigDir)).to.have.members(['bundle.crt', 'csr.pem', 'private.key']);
    });

    it('should not send a request that holds a private key to ZeroSSL', async function it() {
      const request = await generateCertificateRequest({
        publicKey: forge.pki.publicKeyToPem(pair.keys.publicKey),
        privateKey: pair.keyPem,
      }, '1.2.3.4');
      fs.writeFileSync(path.join(sslConfigDir, 'csr.pem'), `${request}${pair.keyPem}`);

      // Renewal starts this many days before the saved certificate expires
      await expect(buildTask.call(this).run({ expirationDays: 90 }))
        .to.be.rejectedWith('--force');

      expect(createCertificate).to.not.have.been.called();
    });

    // ZeroSSL issues for EC keys too, and renewal sends their request as it
    // sends an RSA one.
    it('should renew with the request on disk for an EC key', async function it() {
      const ecKeyPem = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' })
        .privateKey.export({ type: 'sec1', format: 'pem' });
      const ecRequest = createCertificateRequest(ecKeyPem);
      fs.writeFileSync(privateKeyFilePath, ecKeyPem);
      fs.writeFileSync(path.join(sslConfigDir, 'csr.pem'), ecRequest);

      // Renewal starts this many days before the saved certificate expires
      await expect(buildTask.call(this).run({ expirationDays: 90 }))
        .to.be.rejectedWith('a new certificate was requested');

      expect(createCertificate).to.have.been.calledOnceWith(ecRequest);
      expect(generateKeyPair).to.not.have.been.called();
      expect(fs.readFileSync(privateKeyFilePath, 'utf8')).to.equal(ecKeyPem);
    });

    // Installing an issued certificate needs no request, and renewal needs
    // one for the key on disk, whatever its type.
    it('should install an issued certificate for an EC key and write its request', async function it() {
      this.timeout(30000);
      const ec = issueCertificateForKey(
        pair,
        crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' }).privateKey,
        { ip: '1.2.3.4', days: 60 },
      );
      const bundle = `${ec.pem}${pair.pem}`;
      fs.writeFileSync(privateKeyFilePath, ec.keyPem);
      fs.rmSync(bundleFilePath);
      downloadCertificate.resolves(bundle);

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(createCertificate).to.not.have.been.called();
      expect(generateKeyPair).to.not.have.been.called();
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(bundle);
      expect(fs.readFileSync(privateKeyFilePath, 'utf8')).to.equal(ec.keyPem);
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();

      const request = loadCertificateRequest(fs.readFileSync(path.join(sslConfigDir, 'csr.pem'), 'utf8'));
      expect(request.publicKey.export({ type: 'spki', format: 'der' }))
        .to.deep.equal(crypto.createPublicKey(ec.keyPem).export({ type: 'spki', format: 'der' }));
    });

    // Renewal saves the new certificate's ID before it installs the bundle,
    // and the bundle of the certificate it replaces still serves. A retry
    // after a rejected download has to install the saved certificate rather
    // than report the old one installed.
    it('should not report the bundle of the certificate it renews as the saved one', async function it() {
      this.timeout(30000);
      const renewed = issueCertificate({ ip: '1.2.3.4', keys: pair.keys, days: 60 });
      fs.writeFileSync(bundleFilePath, pair.pem);
      fingerprint = new crypto.X509Certificate(renewed.pem).fingerprint;
      downloadCertificate.resolves(renewed.pem.slice(0, renewed.pem.length / 2));

      await expect(buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS }))
        .to.be.rejectedWith('run the obtain command again');

      expect(downloadCertificate).to.have.been.calledOnceWith(CERTIFICATE_ID);
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(pair.pem);
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.false();

      downloadCertificate.resolves(renewed.pem);

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(createCertificate).to.not.have.been.called();
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(renewed.pem);
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();
    });

    it('should leave the bundle of the saved certificate as it is', async function it() {
      fs.writeFileSync(bundleFilePath, pair.pem);
      fingerprint = new crypto.X509Certificate(pair.pem).fingerprint;

      await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS });

      expect(downloadCertificate).to.not.have.been.called();
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.true();
    });

    // A plain obtain finishes the saved certificate. One ZeroSSL does not
    // know leaves nothing to finish, and only a new certificate restores TLS.
    it('should name --force when ZeroSSL does not know the saved certificate', async function it() {
      zeroSslError = Object.assign(
        new Error('Your ZeroSSL certificate ID (hash) could not be found'),
        { code: 2803 },
      );

      const error = await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS })
        .then(() => null, (e) => e);

      expect(error.message).to.include('could not be found')
        .and.include(CERTIFICATE_ID)
        .and.include('--force');
      expect(createCertificate).to.not.have.been.called();
      expect(downloadCertificate).to.not.have.been.called();
    });

    it('should pass on any other error ZeroSSL answers with', async function it() {
      zeroSslError = Object.assign(new Error('Internal error'), { code: 2900 });

      const error = await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS })
        .then(() => null, (e) => e);

      expect(error).to.equal(zeroSslError);
    });

    // A response cut short says nothing about the issuance, and the next
    // obtain asks for the same certificate again.
    it('should not send a download cut short to a new certificate', async function it() {
      this.timeout(30000);
      downloadCertificate.resolves(pair.pem.slice(0, pair.pem.length / 2));

      const error = await buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS })
        .then(() => null, (e) => e);

      expect(error.message).to.include('run the obtain command again')
        .and.not.include('--force');
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(pair.pem.slice(0, pair.pem.length / 2));
    });

    // The bundle is replaced in place, and renewal writes over one the gateway
    // is serving.
    it('should not replace the bundle with a download the key on disk cannot serve', async function it() {
      this.timeout(30000);
      downloadCertificate.resolves(other.pem);

      await expect(buildTask.call(this).run({ expirationDays: Certificate.EXPIRATION_LIMIT_DAYS }))
        .to.be.rejectedWith('--force');

      expect(downloadCertificate).to.have.been.calledOnce();
      expect(createCertificate).to.not.have.been.called();
      expect(fs.readFileSync(bundleFilePath, 'utf8')).to.equal(pair.pem.slice(0, pair.pem.length / 2));
      expect(realConfig.get('platform.gateway.ssl.enabled')).to.be.false();
    });
  });

  describe('domain verification retry', () => {
    let verifyDomain;
    let enquirer;
    let buildTask;

    beforeEach(function beforeEach() {
      config.set = this.sinon.stub();
      verifyDomain = this.sinon.stub().rejects(
        Object.assign(new Error('domain control validation failed'), { code: 1 }),
      );
      enquirer = getEnquirerMock(this.sinon, false);

      buildTask = (sinon) => obtainZeroSSLCertificateTaskFactory(
        sinon.stub().resolves('csr'),
        sinon.stub().resolves({ privateKey: 'private', publicKey: 'public' }),
        sinon.stub().resolves({
          id: 'certificate-id',
          status: 'pending_validation',
          validation: {
            other_methods: {
              '1.2.3.4': {
                file_validation_url_http: 'http://1.2.3.4/.well-known/x',
                file_validation_content: 'content',
              },
            },
          },
        }),
        verifyDomain,
        sinon.stub(),
        sinon.stub(),
        sinon.stub(),
        sinon.stub(),
        verificationServer,
        { joinPath: sinon.stub().returns('/tmp') },
        sinon.stub(),
      );
    });

    /**
     * @param {Object} context
     * @return {Promise}
     */
    function run(context) {
      const tasks = buildTask(this.sinon)(config, {
        onCertificateCreated: this.sinon.stub(),
      });

      tasks.options.injectWrapper = { enquirer };

      return tasks.run({
        force: true, externalIp: '1.2.3.4', apiKey: 'api-key', ...context,
      });
    }

    // This loop was gated only on noRetry, which is fail-open: it prompts
    // unless a caller remembers to say otherwise. The helper's unattended
    // renewal is safe today only because it happens to pass noRetry, and a
    // prompt reached in that container never settles and never releases the
    // config lock - which then blocks every other dashmate command forever.
    it('should not construct a prompt when the session cannot answer', async function it() {
      const error = await run.call(this, {}).catch((e) => e);

      expect(enquirer.prompt).to.not.have.been.called();

      // The verification failure is what the operator needs to see. Reaching
      // the prompt and being refused there would be safe but would replace the
      // real error with a report that dashmate tried to ask a question, so the
      // decision not to retry has to be made before the prompt is reached.
      expect(error.message).to.contain('domain control validation failed');
      expect(error.message).to.not.contain('without a terminal');
    });

    it('should still ask an operator who is at a terminal', async function it() {
      await expect(run.call(this, { interactive: true })).to.be.rejected();

      expect(enquirer.prompt).to.have.been.calledOnce();
    });

    it('should honour no-retry at a terminal', async function it() {
      await expect(run.call(this, { interactive: true, noRetry: true })).to.be.rejected();

      expect(enquirer.prompt).to.not.have.been.called();
    });
  });
});

import fs from 'fs';
import obtainZeroSSLCertificateTaskFactory from '../../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import { ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';
import getEnquirerMock from '../../../../src/test/mock/getEnquirerMock.js';

describe('obtainZeroSSLCertificateTaskFactory', () => {
  let config;
  let verificationServer;
  let validateZeroSslCertificate;
  let obtainZeroSSLCertificateTask;

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('local'),
    };

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
      { joinPath: this.sinon.stub() }, // homeDir
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

  it('should create the ZeroSSL private key with mode 0600', async function it() {
    const homeDir = HomeDir.createTemp();

    try {
      const realConfig = getBaseConfigFactory(homeDir)();
      const privateKeyFilePath = homeDir.joinPath('ssl', 'private.key');
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
        this.sinon.stub().resolves({ privateKey: 'private-key' }),
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

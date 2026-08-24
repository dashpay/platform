import fs from 'fs';
import path from 'path';
import obtainZeroSSLCertificateTaskFactory from '../../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';

describe('obtainZeroSSLCertificateTaskFactory', () => {
  let config;
  let verificationServer;
  let validateZeroSslCertificate;
  let generateCsr;
  let generateKeyPair;
  let createZeroSSLCertificate;
  let verifyDomain;
  let downloadCertificate;
  let getCertificate;
  let listCertificates;
  let saveCertificateTask;
  let homeDir;
  let configFileRepository;
  let configFile;
  let obtainZeroSSLCertificateTask;
  let sslConfigDir;

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
      set: this.sinon.stub(),
      getName: this.sinon.stub().returns('local'),
    };
    config.get
      .withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
      .returns('test-api-key');
    config.get.withArgs('externalIp').returns('1.2.3.4');

    sslConfigDir = path.join('/home/dir', 'local', 'platform', 'gateway', 'ssl');
    homeDir = {
      joinPath: this.sinon.stub().callsFake((...parts) => path.join('/home/dir', ...parts)),
    };

    verificationServer = {
      setup: this.sinon.stub().resolves(),
      start: this.sinon.stub().resolves(),
      stop: this.sinon.stub().resolves(),
      destroy: this.sinon.stub().resolves(),
      waitForServerIsResponding: this.sinon.stub().resolves(true),
    };

    // The first non-init pipeline task calls validateZeroSslCertificate. Rejecting
    // it simulates a mid-pipeline failure (e.g. the ZeroSSL API going down), which
    // is the path that previously left the verification server bound to port 80.
    validateZeroSslCertificate = this.sinon.stub().rejects(new Error('ZeroSSL API unavailable'));

    generateCsr = this.sinon.stub().resolves('CSR_PEM');
    generateKeyPair = this.sinon.stub().resolves({
      privateKey: 'PRIVATE_KEY_PEM',
      publicKey: 'PUBLIC_KEY_PEM',
    });
    createZeroSSLCertificate = this.sinon.stub().resolves({
      id: 'cert-id',
      status: 'issued',
      common_name: '1.2.3.4',
      expires: '2099-01-01 00:00:00',
      validation: { other_methods: { '1.2.3.4': {} } },
    });
    verifyDomain = this.sinon.stub().resolves();
    downloadCertificate = this.sinon.stub().resolves('CERT_BUNDLE_PEM');
    getCertificate = this.sinon.stub();
    listCertificates = this.sinon.stub();
    saveCertificateTask = this.sinon.stub();
    configFileRepository = { write: this.sinon.stub() };
    configFile = {};

    // Prevent the init task from touching the real filesystem.
    this.sinon.stub(fs, 'mkdirSync').returns(undefined);
    this.sinon.stub(fs, 'mkdtempSync').returns('/home/dir/local/platform/gateway/ssl/.zerossl-test');
    this.sinon.stub(fs, 'writeFileSync').returns(undefined);
    this.sinon.stub(fs, 'renameSync').returns(undefined);
    this.sinon.stub(fs, 'rmSync').returns(undefined);

    obtainZeroSSLCertificateTask = obtainZeroSSLCertificateTaskFactory(
      generateCsr,
      generateKeyPair,
      createZeroSSLCertificate,
      verifyDomain,
      downloadCertificate,
      getCertificate,
      listCertificates,
      saveCertificateTask,
      verificationServer,
      homeDir,
      validateZeroSslCertificate,
      configFileRepository,
      configFile,
    );
  });

  it('should stop and destroy the verification server when the pipeline fails', async () => {
    const tasks = obtainZeroSSLCertificateTask(config);

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

  describe('--force mode', () => {
    /**
     * Abort after create is invoked so we never enter the production
     * wait(5000) download-retry loop, while still recording create args.
     */
    function rejectAfterCreate() {
      const sentinel = new Error('STOP_PIPELINE_AFTER_CERT_REQUEST');
      createZeroSSLCertificate.rejects(sentinel);
      return sentinel;
    }

    it('should load externalIp/apiKey/paths, skip validation, and regenerate keypair/CSR/certificate', async () => {
      const sentinel = rejectAfterCreate();
      const tasks = obtainZeroSSLCertificateTask(config);

      let thrownError;
      try {
        await tasks.run({ expirationDays: 30, force: true });
      } catch (e) {
        thrownError = e;
      }
      expect(thrownError).to.equal(sentinel);

      // Existing-certificate short-circuit must be skipped under --force.
      expect(validateZeroSslCertificate).to.not.have.been.called();

      // Required context and paths must still be initialized (#3803 / #4249).
      expect(homeDir.joinPath).to.have.been.calledWith(
        'local',
        'platform',
        'gateway',
        'ssl',
      );
      expect(fs.mkdirSync).to.have.been.calledWith(sslConfigDir, { recursive: true });

      // Keypair/CSR regenerate because force clears isCsrFilePresent.
      expect(generateKeyPair).to.have.been.calledOnce();
      expect(generateCsr).to.have.been.calledOnce();
      // CSR must receive the externalIp loaded from config — previously undefined
      // and crashed node-forge with "Attribute value not specified."
      expect(generateCsr.firstCall.args[1]).to.equal('1.2.3.4');

      // New ZeroSSL certificate requested with loaded externalIp and apiKey.
      expect(createZeroSSLCertificate).to.have.been.calledOnce();
      expect(createZeroSSLCertificate.firstCall.args).to.deep.equal([
        'CSR_PEM',
        '1.2.3.4',
        'test-api-key',
      ]);
    });

    it('should ignore pre-existing presence flags and certificate state under --force', async () => {
      const sentinel = rejectAfterCreate();
      const tasks = obtainZeroSSLCertificateTask(config);

      let thrownError;
      try {
        // Pre-seed flags that would short-circuit generation if force failed
        // to clear them (the #4249 undefined-context / stale-presence path).
        await tasks.run({
          expirationDays: 30,
          force: true,
          isCsrFilePresent: true,
          isPrivateKeyFilePresent: true,
          isBundleFilePresent: true,
          certificate: {
            id: 'old-cert',
            status: 'issued',
            common_name: '1.2.3.4',
          },
        });
      } catch (e) {
        thrownError = e;
      }
      expect(thrownError).to.equal(sentinel);

      expect(validateZeroSslCertificate).to.not.have.been.called();
      // If isCsrFilePresent stayed true, these would not run.
      expect(generateKeyPair).to.have.been.calledOnce();
      expect(generateCsr).to.have.been.calledOnce();
      // If certificate stayed truthy, create would be skipped.
      expect(createZeroSSLCertificate).to.have.been.calledOnce();
    });

    it('should keep the previous certificate configuration when replacement fails', async function it() {
      const configValues = {
        'platform.gateway.ssl.enabled': true,
        'platform.gateway.ssl.provider': 'zerossl',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'test-api-key',
        'platform.gateway.ssl.providerConfigs.zerossl.id': 'old-cert-id',
        externalIp: '1.2.3.4',
      };
      config.get.callsFake((configPath) => configValues[configPath]);
      config.set.callsFake((configPath, value) => {
        configValues[configPath] = value;
      });

      createZeroSSLCertificate.resolves({
        id: 'replacement-cert-id',
        status: 'pending_validation',
        validation: {
          other_methods: {
            '1.2.3.4': {
              file_validation_url_http: 'http://1.2.3.4/.well-known/pki-validation/file',
              file_validation_content: ['validation-content'],
            },
          },
        },
      });
      const replacementFailure = new Error('verification setup failed');
      verificationServer.setup.rejects(replacementFailure);

      const forceTasks = obtainZeroSSLCertificateTask(config);
      let thrownError;
      try {
        await forceTasks.run({ expirationDays: 30, force: true });
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).to.equal(replacementFailure);
      expect(configValues['platform.gateway.ssl.providerConfigs.zerossl.id'])
        .to.equal('old-cert-id');
      expect(configFileRepository.write).to.not.have.been.called();

      verificationServer.setup.resolves();
      validateZeroSslCertificate.resolves({
        data: {
          certificate: {
            id: configValues['platform.gateway.ssl.providerConfigs.zerossl.id'],
            status: 'issued',
            expires: '2099-01-01 00:00:00',
          },
          isCsrFilePresent: true,
          isPrivateKeyFilePresent: true,
          isBundleFilePresent: true,
        },
      });

      const retryTasks = obtainZeroSSLCertificateTask(config);
      await retryTasks.run({ expirationDays: 30 });

      expect(validateZeroSslCertificate).to.have.been.calledOnce();
      expect(generateKeyPair).to.have.been.calledOnce();
      expect(generateCsr).to.have.been.calledOnce();
      expect(createZeroSSLCertificate).to.have.been.calledOnce();
      expect(configValues['platform.gateway.ssl.providerConfigs.zerossl.id'])
        .to.equal('old-cert-id');
    });

    it('should restore previous artifacts and configuration when persistence fails', async function it() {
      const configValues = {
        'platform.gateway.ssl.enabled': true,
        'platform.gateway.ssl.provider': 'zerossl',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'test-api-key',
        'platform.gateway.ssl.providerConfigs.zerossl.id': 'old-cert-id',
        externalIp: '1.2.3.4',
      };
      config.get.callsFake((configPath) => configValues[configPath]);
      config.set.callsFake((configPath, value) => {
        configValues[configPath] = value;
      });

      const previousArtifacts = {
        [path.join(sslConfigDir, 'private.key')]: 'OLD_PRIVATE_KEY_PEM',
        [path.join(sslConfigDir, 'csr.pem')]: 'OLD_CSR_PEM',
        [path.join(sslConfigDir, 'bundle.crt')]: 'OLD_CERT_BUNDLE_PEM',
      };
      this.sinon.stub(fs, 'existsSync').callsFake(
        (filePath) => Object.hasOwn(previousArtifacts, filePath),
      );
      this.sinon.stub(fs, 'readFileSync').callsFake((filePath) => previousArtifacts[filePath]);

      const persistenceFailure = new Error('config persistence failed');
      configFileRepository.write.onFirstCall().throws(persistenceFailure);

      const clock = this.sinon.useFakeTimers();
      const tasks = obtainZeroSSLCertificateTask(config);
      let thrownError;
      const runPromise = tasks.run({ expirationDays: 30, force: true }).catch((e) => {
        thrownError = e;
      });
      await clock.tickAsync(5000);
      await runPromise;

      expect(thrownError).to.equal(persistenceFailure);
      expect(configValues['platform.gateway.ssl.providerConfigs.zerossl.id'])
        .to.equal('old-cert-id');
      expect(configFileRepository.write).to.have.been.calledTwice();
      Object.entries(previousArtifacts).forEach(([filePath, content]) => {
        expect(fs.writeFileSync).to.have.been.calledWith(filePath, content, 'utf8');
      });
    });

    it('should overwrite existing artifacts in place to preserve bind-mounted inodes', async function it() {
      const configValues = {
        'platform.gateway.ssl.enabled': true,
        'platform.gateway.ssl.provider': 'zerossl',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'test-api-key',
        'platform.gateway.ssl.providerConfigs.zerossl.id': 'old-cert-id',
        externalIp: '1.2.3.4',
      };
      config.get.callsFake((configPath) => configValues[configPath]);
      config.set.callsFake((configPath, value) => {
        configValues[configPath] = value;
      });

      const previousArtifacts = {
        [path.join(sslConfigDir, 'private.key')]: 'OLD_PRIVATE_KEY_PEM',
        [path.join(sslConfigDir, 'csr.pem')]: 'OLD_CSR_PEM',
        [path.join(sslConfigDir, 'bundle.crt')]: 'OLD_CERT_BUNDLE_PEM',
      };
      this.sinon.stub(fs, 'existsSync').callsFake(
        (filePath) => Object.hasOwn(previousArtifacts, filePath),
      );
      this.sinon.stub(fs, 'readFileSync').callsFake((filePath) => previousArtifacts[filePath]);

      const clock = this.sinon.useFakeTimers();
      const tasks = obtainZeroSSLCertificateTask(config);
      const runPromise = tasks.run({ expirationDays: 30, force: true });
      await clock.tickAsync(5000);
      await runPromise;

      // Replacement contents must land on the existing destination inodes,
      // which the gateway container's single-file bind mounts stay attached to.
      expect(fs.writeFileSync).to.have.been.calledWith(path.join(sslConfigDir, 'private.key'), 'PRIVATE_KEY_PEM', 'utf8');
      expect(fs.writeFileSync).to.have.been.calledWith(path.join(sslConfigDir, 'csr.pem'), 'CSR_PEM', 'utf8');
      expect(fs.writeFileSync).to.have.been.calledWith(path.join(sslConfigDir, 'bundle.crt'), 'CERT_BUNDLE_PEM', 'utf8');
      // renameSync would create new inodes the running container never sees.
      expect(fs.renameSync).to.not.have.been.called();
    });

    it('should stage the private key with owner-only permissions', async function it() {
      const configValues = {
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'test-api-key',
        externalIp: '1.2.3.4',
      };
      config.get.callsFake((configPath) => configValues[configPath]);
      config.set.callsFake((configPath, value) => {
        configValues[configPath] = value;
      });

      this.sinon.stub(fs, 'existsSync').returns(false);
      this.sinon.stub(fs, 'readFileSync').returns(undefined);

      const clock = this.sinon.useFakeTimers();
      const tasks = obtainZeroSSLCertificateTask(config);
      const runPromise = tasks.run({ expirationDays: 30, force: true });
      await clock.tickAsync(5000);
      await runPromise;

      const stagingDir = '/home/dir/local/platform/gateway/ssl/.zerossl-test';
      expect(fs.writeFileSync).to.have.been.calledWith(
        path.join(stagingDir, 'private.key'),
        'PRIVATE_KEY_PEM',
        { encoding: 'utf8', mode: 0o600 },
      );
      // CSR and certificate bundle keep the default mode
      expect(fs.writeFileSync).to.have.been.calledWith(
        path.join(stagingDir, 'csr.pem'),
        'CSR_PEM',
        { encoding: 'utf8', mode: undefined },
      );
      expect(fs.writeFileSync).to.have.been.calledWith(
        path.join(stagingDir, 'bundle.crt'),
        'CERT_BUNDLE_PEM',
        { encoding: 'utf8', mode: undefined },
      );
    });

    it('should not persist replacement configuration when artifact installation fails', async function it() {
      const configValues = {
        'platform.gateway.ssl.enabled': true,
        'platform.gateway.ssl.provider': 'zerossl',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey': 'test-api-key',
        'platform.gateway.ssl.providerConfigs.zerossl.id': 'old-cert-id',
        externalIp: '1.2.3.4',
      };
      config.get.callsFake((configPath) => configValues[configPath]);
      config.set.callsFake((configPath, value) => {
        configValues[configPath] = value;
      });

      const artifactFailure = new Error('artifact installation failed');
      fs.renameSync.onSecondCall().throws(artifactFailure);

      const clock = this.sinon.useFakeTimers();
      const tasks = obtainZeroSSLCertificateTask(config);
      let thrownError;
      const runPromise = tasks.run({ expirationDays: 30, force: true }).catch((e) => {
        thrownError = e;
      });
      await clock.tickAsync(5000);
      await runPromise;

      expect(thrownError).to.equal(artifactFailure);
      expect(configValues['platform.gateway.ssl.providerConfigs.zerossl.id'])
        .to.equal('old-cert-id');
      expect(configFileRepository.write).to.not.have.been.called();
    });

    it('should fail with the missing-API-key error before any network or generation work', async () => {
      config.get
        .withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
        .returns(undefined);

      const tasks = obtainZeroSSLCertificateTask(config);

      let thrownError;
      try {
        await tasks.run({ expirationDays: 30, force: true });
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).to.be.an('error');
      expect(thrownError.message).to.match(/ZeroSSL API key is not set/);

      expect(validateZeroSslCertificate).to.not.have.been.called();
      expect(generateKeyPair).to.not.have.been.called();
      expect(generateCsr).to.not.have.been.called();
      expect(createZeroSSLCertificate).to.not.have.been.called();
    });

    it('should fail with the missing-external-IP error before any network or generation work', async () => {
      config.get.withArgs('externalIp').returns(undefined);

      const tasks = obtainZeroSSLCertificateTask(config);

      let thrownError;
      try {
        await tasks.run({ expirationDays: 30, force: true });
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).to.be.an('error');
      expect(thrownError.message).to.match(/External IP is not set/);

      expect(validateZeroSslCertificate).to.not.have.been.called();
      expect(generateKeyPair).to.not.have.been.called();
      expect(generateCsr).to.not.have.been.called();
      expect(createZeroSSLCertificate).to.not.have.been.called();
    });
  });
});

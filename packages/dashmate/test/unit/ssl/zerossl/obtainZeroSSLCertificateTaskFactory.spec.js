import fs from 'fs';
import obtainZeroSSLCertificateTaskFactory from '../../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import ConfigFile from '../../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../../src/config/configFile/ConfigFileJsonRepository.js';
import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import { ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';

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
      { write: this.sinon.stub() }, // configFileRepository
      {}, // configFile
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

  it('should leave SSL settings dirty for command finalization after the mid-command save', async function it() {
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
        repository,
        configFile,
      )(realConfig);

      await task.run({ expirationDays: 30 });

      expect(realConfig.isChanged()).to.be.true();
      expect(repository.read().getConfig(realConfig.getName())
        .get('platform.gateway.ssl.provider')).to.equal('zerossl');
    } finally {
      homeDir.remove();
    }
  });
});

import obtainZeroSSLCertificateTaskFactory from '../../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';

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
});

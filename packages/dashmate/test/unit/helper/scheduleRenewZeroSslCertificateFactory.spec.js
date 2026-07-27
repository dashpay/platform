import scheduleRenewZeroSslCertificateFactory from '../../../src/helper/scheduleRenewZeroSslCertificateFactory.js';

describe('scheduleRenewZeroSslCertificateFactory', () => {
  let config;
  let getCertificate;
  let obtainZeroSSLCertificateTask;
  let dockerCompose;
  let configFileRepository;
  let configFile;
  let writeConfigTemplates;
  let scheduleRenewZeroSslCertificate;

  beforeEach(function beforeEach() {
    config = {
      get: this.sinon.stub(),
    };

    getCertificate = this.sinon.stub();
    obtainZeroSSLCertificateTask = this.sinon.stub();
    dockerCompose = { execCommand: this.sinon.stub() };
    configFileRepository = { write: this.sinon.stub() };
    configFile = {};
    writeConfigTemplates = this.sinon.stub();

    scheduleRenewZeroSslCertificate = scheduleRenewZeroSslCertificateFactory(
      getCertificate,
      obtainZeroSSLCertificateTask,
      dockerCompose,
      configFileRepository,
      configFile,
      writeConfigTemplates,
    );
  });

  describe('certificate read failure', () => {
    // Regression test for the December outage: when the ZeroSSL API is down while
    // reading the current certificate, the scheduler used to reject. Because it is
    // re-invoked fire-and-forget from the cron completion callback, that rejection
    // went unhandled and crashed the helper instead of backing off — silently
    // disabling renewal until the certificate expired.
    it('should not reject and should arm a 1 hour retry when the ZeroSSL API is unavailable', async function it() {
      const setTimeoutStub = this.sinon.stub(global, 'setTimeout');
      const consoleErrorStub = this.sinon.stub(console, 'error');

      getCertificate.rejects(new Error('ZeroSSL API unavailable'));

      // Must resolve — a rejection here is the crash that defeated renewal in December.
      await scheduleRenewZeroSslCertificate(config);

      expect(getCertificate).to.have.been.calledOnce();
      expect(consoleErrorStub).to.have.been.calledOnce();
      expect(setTimeoutStub).to.have.been.calledOnce();
      expect(setTimeoutStub.firstCall.args[1]).to.equal(60 * 60 * 1000);
    });

    it('should arm a 1 hour retry when the certificate is not found', async function it() {
      const setTimeoutStub = this.sinon.stub(global, 'setTimeout');
      this.sinon.stub(console, 'error');

      getCertificate.resolves(null);

      await scheduleRenewZeroSslCertificate(config);

      expect(setTimeoutStub).to.have.been.calledOnce();
      expect(setTimeoutStub.firstCall.args[1]).to.equal(60 * 60 * 1000);
    });
  });
});

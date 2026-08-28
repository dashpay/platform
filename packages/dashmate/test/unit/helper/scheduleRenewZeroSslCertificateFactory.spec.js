import scheduleRenewZeroSslCertificateFactory from '../../../src/helper/scheduleRenewZeroSslCertificateFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import RenewalRecordRepository from '../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import ConfigIsNotPresentError from '../../../src/config/errors/ConfigIsNotPresentError.js';
import Certificate from '../../../src/ssl/zerossl/Certificate.js';
import { CONFIG_REFRESH_INTERVAL_MS } from '../../../src/helper/watchCertificateConfig.js';

describe('scheduleRenewZeroSslCertificateFactory', () => {
  let config;
  let getCertificate;
  let obtainZeroSSLCertificateTask;
  let dockerCompose;
  let configFileRepository;
  let writeConfigTemplates;
  let homeDir;
  let scheduleRenewZeroSslCertificate;

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('base'),
      isChanged: this.sinon.stub().returns(false),
    };

    getCertificate = this.sinon.stub();
    obtainZeroSSLCertificateTask = this.sinon.stub();
    dockerCompose = { execCommand: this.sinon.stub() };
    configFileRepository = {
      read: this.sinon.stub().returns({
        getConfig: this.sinon.stub().returns(config),
      }),
      acquire: this.sinon.stub(),
      isExclusive: () => true,
      readAndMigrate: this.sinon.stub().returns({
        configFile: {
          getConfig: this.sinon.stub().returns(config),
        },
      }),
      write: this.sinon.stub(),
      release: this.sinon.stub(),
    };
    writeConfigTemplates = this.sinon.stub();
    config.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    config.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');
    config.get.withArgs('externalIp').returns('127.0.0.1');
    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
      .returns('api-key');
    config.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.id')
      .returns('certificate-id');

    scheduleRenewZeroSslCertificate = scheduleRenewZeroSslCertificateFactory(
      getCertificate,
      obtainZeroSSLCertificateTask,
      dockerCompose,
      configFileRepository,
      writeConfigTemplates,
      homeDir,
      new RenewalRecordRepository(homeDir),
    );
  });

  afterEach(() => {
    homeDir.remove();
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

    it('should preserve provider handoff across a retry', async function it() {
      const setTimeoutStub = this.sinon.stub(global, 'setTimeout');
      const onConfigurationChanged = this.sinon.stub().resolves(true);
      this.sinon.stub(console, 'error');
      getCertificate.rejects(new Error('ZeroSSL API unavailable'));

      await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);

      getCertificate.resetBehavior();
      config.get.withArgs('platform.gateway.ssl.provider').returns('letsencrypt');
      await setTimeoutStub.firstCall.args[0]();

      expect(onConfigurationChanged).to.have.been.calledOnceWith(config);
    });
  });

  describe('configuration refresh', () => {
    it('should hand off when the config was removed before scheduling completed', async function it() {
      const setTimeoutStub = this.sinon.stub(global, 'setTimeout');
      const onConfigurationChanged = this.sinon.stub().resolves();
      configFileRepository.read.returns({
        getConfig: this.sinon.stub().throws(new ConfigIsNotPresentError('base')),
      });

      await expect(scheduleRenewZeroSslCertificate(config, onConfigurationChanged))
        .to.not.be.rejected();

      expect(getCertificate).to.not.have.been.called();
      expect(setTimeoutStub).to.not.have.been.called();
      expect(onConfigurationChanged).to.have.been.calledOnceWith(null);
    });

    it('should retry when the current config cannot be read', async function it() {
      const setTimeoutStub = this.sinon.stub(global, 'setTimeout');
      this.sinon.stub(console, 'error');
      configFileRepository.read.throws(new Error('read failed'));

      await expect(scheduleRenewZeroSslCertificate(config)).to.not.be.rejected();

      expect(setTimeoutStub).to.have.been.calledOnce();
      expect(setTimeoutStub.firstCall.args[1]).to.equal(60 * 60 * 1000);
    });

    it('should hand off scheduling when the configured provider changed', async function it() {
      const onConfigurationChanged = this.sinon.stub().resolves(true);
      config.get.withArgs('platform.gateway.ssl.provider').returns('letsencrypt');

      await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);

      expect(onConfigurationChanged).to.have.been.calledOnceWith(config);
      expect(getCertificate).to.not.have.been.called();
    });

    it('should detect a provider change before the old certificate renewal date', async function it() {
      const clock = this.sinon.useFakeTimers({
        now: new Date('2026-07-31T00:00:00.000Z'),
      });
      const onConfigurationChanged = this.sinon.stub().resolves(true);
      getCertificate.resolves({
        id: 'certificate-id',
        status: 'issued',
        expires: new Date('2026-10-01T00:00:00.000Z'),
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);

      config.get.withArgs('platform.gateway.ssl.provider').returns('letsencrypt');
      await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);

      expect(onConfigurationChanged).to.have.been.calledOnceWith(config);
      expect(obtainZeroSSLCertificateTask).to.not.have.been.called();

      // The handoff must leave nothing armed for ZeroSSL, so running every
      // remaining timer proves the old renewal can never fire. Ticking a blanket
      // 64 days here instead would replay ~92k config-refresh firings (one real
      // event-loop hop each) whenever a poll timer survives, blowing the test
      // timeout on a slow runner rather than failing on the assertion below.
      await clock.runAllAsync();

      expect(onConfigurationChanged).to.have.been.calledOnce();
      expect(obtainZeroSSLCertificateTask).to.not.have.been.called();
    });

    it('should resume scheduling after SSL is disabled and re-enabled', async function it() {
      const clock = this.sinon.useFakeTimers({
        now: new Date('2026-07-31T00:00:00.000Z'),
      });
      const onConfigurationChanged = this.sinon.stub();
      onConfigurationChanged.onFirstCall().resolves(false);
      onConfigurationChanged.onSecondCall().resolves(true);
      getCertificate.resolves({
        id: 'certificate-id',
        status: 'issued',
        expires: new Date('2026-10-01T00:00:00.000Z'),
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);

      config.get.withArgs('platform.gateway.ssl.enabled').returns(false);
      await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);
      config.get.withArgs('platform.gateway.ssl.enabled').returns(true);
      await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);

      expect(onConfigurationChanged).to.have.been.calledTwice();
      expect(onConfigurationChanged.firstCall).to.have.been.calledWith(config);
      expect(onConfigurationChanged.secondCall).to.have.been.calledWith(config);
    });

    it('should keep watching after removal and schedule a recreated config', async function it() {
      const clock = this.sinon.useFakeTimers({
        now: new Date('2026-07-31T00:00:00.000Z'),
      });
      const onConfigurationChanged = this.sinon.stub().resolves(true);
      const removedConfigFile = {
        getConfig: this.sinon.stub().throws(new ConfigIsNotPresentError('base')),
      };
      const recreatedConfig = {
        getName: this.sinon.stub().returns('base'),
        get: this.sinon.stub(),
      };
      recreatedConfig.get.withArgs('platform.gateway.ssl.enabled').returns(true);
      recreatedConfig.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');
      recreatedConfig.get.withArgs('externalIp').returns('127.0.0.1');
      recreatedConfig.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
        .returns('api-key');
      recreatedConfig.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.id')
        .returns('recreated-certificate-id');
      configFileRepository.read.onCall(1).returns(removedConfigFile);
      configFileRepository.read.onCall(2).returns({
        getConfig: this.sinon.stub().returns(recreatedConfig),
      });
      getCertificate.resolves({
        id: 'certificate-id',
        status: 'issued',
        expires: new Date('2026-10-01T00:00:00.000Z'),
        isExpiredInDays: this.sinon.stub().returns(false),
      });

      await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);
      await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);

      expect(onConfigurationChanged).to.not.have.been.called();

      await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);

      expect(onConfigurationChanged).to.have.been.calledOnceWith(recreatedConfig);
    });

    [
      ['external IP', 'externalIp', '127.0.0.2'],
      [
        'API key',
        'platform.gateway.ssl.providerConfigs.zerossl.apiKey',
        'new-api-key',
      ],
      [
        'certificate ID',
        'platform.gateway.ssl.providerConfigs.zerossl.id',
        'new-certificate-id',
      ],
    ].forEach(([name, path, value]) => {
      it(`should reschedule when the ZeroSSL ${name} changes`, async function it() {
        const clock = this.sinon.useFakeTimers({
          now: new Date('2026-07-31T00:00:00.000Z'),
        });
        const onConfigurationChanged = this.sinon.stub().resolves(true);
        const changedConfig = {
          getName: this.sinon.stub().returns('base'),
          get: this.sinon.stub(),
        };
        changedConfig.get.withArgs('platform.gateway.ssl.enabled').returns(true);
        changedConfig.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');
        changedConfig.get.withArgs('externalIp').returns('127.0.0.1');
        changedConfig.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.apiKey')
          .returns('api-key');
        changedConfig.get.withArgs('platform.gateway.ssl.providerConfigs.zerossl.id')
          .returns('certificate-id');
        changedConfig.get.withArgs(path).returns(value);
        getCertificate.resolves({
          id: 'certificate-id',
          status: 'issued',
          expires: new Date('2026-10-01T00:00:00.000Z'),
          isExpiredInDays: this.sinon.stub().returns(false),
        });

        await scheduleRenewZeroSslCertificate(config, onConfigurationChanged);
        configFileRepository.read.returns({
          getConfig: this.sinon.stub().returns(changedConfig),
        });
        await clock.tickAsync(CONFIG_REFRESH_INTERVAL_MS);

        expect(onConfigurationChanged).to.have.been.calledOnceWith(changedConfig);
      });
    });
  });

  it('should run renewal through the fresh locked configuration path', async function it() {
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-07-31T00:00:00.000Z'),
    });
    const run = this.sinon.stub().resolves();
    obtainZeroSSLCertificateTask.returns({ run });
    config.isChanged.returns(true);
    getCertificate.resolves({
      id: 'certificate-id',
      status: 'issued',
      expires: new Date('2026-08-01T00:00:00.000Z'),
      isExpiredInDays: this.sinon.stub().returns(true),
    });

    await scheduleRenewZeroSslCertificate(config);
    await clock.tickAsync(3001);

    expect(configFileRepository.acquire).to.have.been.calledOnce();
    expect(configFileRepository.readAndMigrate).to.have.been.calledOnce();
    expect(obtainZeroSSLCertificateTask).to.have.been.calledOnceWith(config);
    expect(run).to.have.been.calledOnceWith({
      expirationDays: Certificate.EXPIRATION_LIMIT_DAYS,
      noRetry: true,
      renewalGeneration: 1,
    });
    expect(configFileRepository.write).to.have.been.calledOnce();
    expect(writeConfigTemplates).to.have.been.calledOnceWith(config);
    expect(configFileRepository.release).to.have.been.calledOnce();
    expect(dockerCompose.execCommand)
      .to.have.been.calledOnceWith(config, 'gateway', 'kill -SIGHUP 1');
    expect(getCertificate).to.have.been.calledTwice();
  });

  it('should resume a pending certificate without waiting for its expiry', async function it() {
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-07-31T00:00:00.000Z'),
    });
    const run = this.sinon.stub().resolves();
    obtainZeroSSLCertificateTask.returns({ run });
    getCertificate.resolves({
      id: 'pending-certificate-id',
      status: 'pending_validation',
      expires: new Date('2026-10-01T00:00:00.000Z'),
      isExpiredInDays: this.sinon.stub().returns(false),
    });

    await scheduleRenewZeroSslCertificate(config);
    await clock.tickAsync(3001);

    expect(run).to.have.been.calledOnce();
  });

  it('should resume obtaining a pending certificate that has no expiry date', async function it() {
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-08-19T00:00:00.000Z'),
    });
    this.sinon.stub(console, 'log');

    const certificate = {
      id: 'pending-certificate-id',
      status: 'pending_validation',
      expires: null,
      isExpiredInDays: this.sinon.stub().returns(false),
    };
    const tasks = {
      run: this.sinon.stub().resolves(),
    };

    getCertificate.resolves(certificate);
    obtainZeroSSLCertificateTask.returns(tasks);

    await scheduleRenewZeroSslCertificate(config);
    await clock.tickAsync(3000);

    expect(obtainZeroSSLCertificateTask).to.have.been.calledOnceWith(config);
    expect(tasks.run).to.have.been.calledOnceWithExactly({
      expirationDays: 3,
      noRetry: true,
      renewalGeneration: 1,
    });
  });

  // A missing expiry date is not a renewal time. Scheduling from it yields
  // 1970-01-01, and cron rejects a date in the past by throwing, which takes down
  // the helper's only renewal chain instead of renewing the certificate.
  it('should obtain immediately when an issued certificate has no expiry date', async function it() {
    const clock = this.sinon.useFakeTimers({
      now: new Date('2026-08-19T00:00:00.000Z'),
    });
    this.sinon.stub(console, 'log');

    const run = this.sinon.stub().resolves();
    obtainZeroSSLCertificateTask.returns({ run });
    getCertificate.resolves({
      id: 'issued-without-expiry',
      status: 'issued',
      expires: null,
      isExpiredInDays: this.sinon.stub().returns(false),
    });

    await expect(scheduleRenewZeroSslCertificate(config)).to.not.be.rejected();

    await clock.tickAsync(3001);

    expect(run).to.have.been.calledOnceWith({
      expirationDays: Certificate.EXPIRATION_LIMIT_DAYS,
      noRetry: true,
      renewalGeneration: 1,
    });
  });
});

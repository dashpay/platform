import scheduleRenewCertificate from '../../../src/helper/scheduleRenewCertificate.js';

describe('scheduleRenewCertificate', () => {
  it('should hand provider changes to the newly configured scheduler', async function it() {
    const zeroSslConfig = {
      get: this.sinon.stub(),
    };
    zeroSslConfig.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    zeroSslConfig.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');

    const letsEncryptConfig = {
      get: this.sinon.stub(),
    };
    letsEncryptConfig.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    letsEncryptConfig.get.withArgs('platform.gateway.ssl.provider').returns('letsencrypt');

    let configurationChanged;
    const scheduleRenewZeroSslCertificate = this.sinon.stub()
      .callsFake(async (config, onConfigurationChanged) => {
        configurationChanged = onConfigurationChanged;
      });
    const scheduleRenewLetsEncryptCertificate = this.sinon.stub().resolves();
    const watchInactiveConfig = this.sinon.stub();

    await scheduleRenewCertificate(
      zeroSslConfig,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );
    await configurationChanged(letsEncryptConfig);

    expect(scheduleRenewZeroSslCertificate).to.have.been.calledOnceWith(
      zeroSslConfig,
      this.sinon.match.func,
    );
    expect(scheduleRenewLetsEncryptCertificate).to.have.been.calledOnceWith(
      letsEncryptConfig,
      this.sinon.match.func,
    );
  });

  it('should watch a disabled provider', async function it() {
    const config = {
      get: this.sinon.stub(),
    };
    config.get.withArgs('platform.gateway.ssl.enabled').returns(false);
    const scheduleRenewZeroSslCertificate = this.sinon.stub();
    const scheduleRenewLetsEncryptCertificate = this.sinon.stub();
    const watchInactiveConfig = this.sinon.stub();

    const result = await scheduleRenewCertificate(
      config,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );

    expect(result).to.be.undefined();
    expect(scheduleRenewZeroSslCertificate).to.not.have.been.called();
    expect(scheduleRenewLetsEncryptCertificate).to.not.have.been.called();
    expect(watchInactiveConfig).to.have.been.calledOnceWith(config, this.sinon.match.func);
  });

  it('should watch an initially disabled config and schedule it when enabled', async function it() {
    const disabledConfig = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('base'),
    };
    disabledConfig.get.withArgs('platform.gateway.ssl.enabled').returns(false);

    const enabledConfig = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('base'),
    };
    enabledConfig.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    enabledConfig.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');

    let onActivated;
    const watchInactiveConfig = this.sinon.stub()
      .callsFake((config, callback) => {
        onActivated = callback;
      });
    const scheduleRenewZeroSslCertificate = this.sinon.stub().resolves();
    const scheduleRenewLetsEncryptCertificate = this.sinon.stub();

    const result = await scheduleRenewCertificate(
      disabledConfig,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );
    await onActivated(enabledConfig);

    expect(result).to.be.undefined();
    expect(watchInactiveConfig).to.have.been.calledOnceWith(
      disabledConfig,
      this.sinon.match.func,
    );
    expect(scheduleRenewZeroSslCertificate).to.have.been.calledOnceWith(
      enabledConfig,
      this.sinon.match.func,
    );
  });

  it('should watch for recreation when a config disappears during provider handoff', async function it() {
    const config = {
      get: this.sinon.stub(),
      getName: this.sinon.stub().returns('base'),
    };
    config.get.withArgs('platform.gateway.ssl.enabled').returns(true);
    config.get.withArgs('platform.gateway.ssl.provider').returns('zerossl');
    const scheduleRenewZeroSslCertificate = this.sinon.stub()
      .callsFake(async (scheduledConfig, onConfigurationChanged) => {
        await onConfigurationChanged(null);
      });
    const scheduleRenewLetsEncryptCertificate = this.sinon.stub();
    const watchInactiveConfig = this.sinon.stub();

    await scheduleRenewCertificate(
      config,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );

    expect(watchInactiveConfig).to.have.been.calledOnceWith(config, this.sinon.match.func);
  });
});

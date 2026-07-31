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

    await scheduleRenewCertificate(
      zeroSslConfig,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
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

  it('should stop scheduling for a disabled provider', async function it() {
    const config = {
      get: this.sinon.stub(),
    };
    config.get.withArgs('platform.gateway.ssl.enabled').returns(false);
    const scheduleRenewZeroSslCertificate = this.sinon.stub();
    const scheduleRenewLetsEncryptCertificate = this.sinon.stub();

    const scheduled = await scheduleRenewCertificate(
      config,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
    );

    expect(scheduled).to.be.false();
    expect(scheduleRenewZeroSslCertificate).to.not.have.been.called();
    expect(scheduleRenewLetsEncryptCertificate).to.not.have.been.called();
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

    const scheduled = await scheduleRenewCertificate(
      disabledConfig,
      scheduleRenewZeroSslCertificate,
      scheduleRenewLetsEncryptCertificate,
      watchInactiveConfig,
    );
    await onActivated(enabledConfig);

    expect(scheduled).to.be.true();
    expect(watchInactiveConfig).to.have.been.calledOnceWith(
      disabledConfig,
      this.sinon.match.func,
    );
    expect(scheduleRenewZeroSslCertificate).to.have.been.calledOnceWith(
      enabledConfig,
      this.sinon.match.func,
    );
  });
});

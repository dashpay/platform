import fs from 'fs';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../src/config/configFile/ConfigFileJsonRepository.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renewCertificate from '../../../src/helper/renewCertificate.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';

describe('renewCertificate', () => {
  let homeDir;
  let repository;
  let configName;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();

    const config = getBaseConfigFactory(homeDir)();
    configName = config.getName();
    config.set('platform.gateway.ssl.enabled', true);
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.set(
      'platform.gateway.ssl.providerConfigs.zerossl.apiKey',
      'api-key-000000000000000000000000',
    );
    config.set(
      'platform.gateway.ssl.providerConfigs.zerossl.id',
      'old-certificate-id-00000000000000',
    );

    const configFile = new ConfigFile(
      [config],
      '4.1.0',
      'abcdef12',
      configName,
      null,
    );

    repository = new ConfigFileJsonRepository(
      (data) => data,
      homeDir,
      () => null,
    );
    repository.write(configFile);
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should preserve an unrelated update made after helper startup', async function it() {
    repository.update((configFile) => {
      configFile.getConfig(configName).set('description', 'updated after helper startup');
    });

    let renewedConfig;
    const obtainCertificateTask = this.sinon.stub().callsFake((config) => {
      renewedConfig = config;

      return {
        run: this.sinon.stub().callsFake(async () => {
          expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.true();
          config.set(
            'platform.gateway.ssl.providerConfigs.zerossl.id',
            'renewed-certificate-id-0000000000',
          );
        }),
      };
    });
    const writeConfigTemplates = this.sinon.stub().callsFake((config) => {
      expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.true();
      config.markAsSaved();
    });

    const result = await renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    });

    const persisted = repository.read().getConfig(configName);

    expect(result).to.deep.equal({
      config: renewedConfig,
      renewed: true,
    });
    expect(persisted.get('description')).to.equal('updated after helper startup');
    expect(persisted.get('platform.gateway.ssl.providerConfigs.zerossl.id'))
      .to.equal('renewed-certificate-id-0000000000');
    expect(obtainCertificateTask).to.have.been.calledOnce();
    expect(writeConfigTemplates).to.have.been.calledOnceWith(renewedConfig);
    expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.false();
  });

  it('should not renew when the provider changed after helper startup', async function it() {
    repository.update((configFile) => {
      configFile.getConfig(configName).set('platform.gateway.ssl.provider', 'self-signed');
    });

    const obtainCertificateTask = this.sinon.stub();
    const writeConfigTemplates = this.sinon.stub();

    const result = await renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    });

    expect(result.renewed).to.be.false();
    expect(result.config.getName()).to.equal(configName);
    expect(obtainCertificateTask).to.not.have.been.called();
    expect(writeConfigTemplates).to.not.have.been.called();
    expect(repository.read().getConfig(configName)
      .get('platform.gateway.ssl.provider')).to.equal('self-signed');
    expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.false();
  });

  it('should not write or render when the obtain task changes nothing', async function it() {
    const write = this.sinon.spy(repository, 'write');
    const obtainCertificateTask = this.sinon.stub().returns({
      run: this.sinon.stub().resolves(),
    });
    const writeConfigTemplates = this.sinon.stub();

    await renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    });

    expect(write).to.not.have.been.called();
    expect(writeConfigTemplates).to.not.have.been.called();
  });

  // Issuance runs for minutes, long enough for the lease to be stolen and
  // another command to save and render newer state. Rendering from this
  // configuration would overwrite it, and the save's own check is too late.
  it('should not render service files when the lease was lost during issuance', async function it() {
    const obtainCertificateTask = this.sinon.stub().callsFake((config) => ({
      run: this.sinon.stub().callsFake(async () => {
        config.set(
          'platform.gateway.ssl.providerConfigs.zerossl.id',
          'issued-certificate-id-000000000000',
        );

        // The lock went stale while the certificate was being issued.
        repository.isExclusive = () => false;
      }),
    }));
    const writeConfigTemplates = this.sinon.stub();

    await expect(renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    })).to.be.rejectedWith('Lost the configuration lock');

    expect(writeConfigTemplates).to.not.have.been.called();
  });

  it('should checkpoint produced certificate state when obtain fails', async function it() {
    const obtainCertificateTask = this.sinon.stub().callsFake((config, {
      onCertificateCreated,
    }) => ({
      run: this.sinon.stub().callsFake(async () => {
        config.set(
          'platform.gateway.ssl.providerConfigs.zerossl.id',
          'pending-certificate-id-00000000000',
        );
        onCertificateCreated();

        expect(repository.read().getConfig(configName)
          .get('platform.gateway.ssl.providerConfigs.zerossl.id'))
          .to.equal('pending-certificate-id-00000000000');

        throw new Error('verification failed');
      }),
    }));
    const writeConfigTemplates = this.sinon.stub();

    await expect(renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    })).to.be.rejectedWith('verification failed');

    const persisted = repository.read().getConfig(configName);

    expect(persisted.get('platform.gateway.ssl.providerConfigs.zerossl.id'))
      .to.equal('pending-certificate-id-00000000000');
    expect(persisted.get('platform.gateway.ssl.enabled')).to.be.true();
    expect(writeConfigTemplates).to.not.have.been.called();
    expect(fs.existsSync(homeDir.joinPath('config.json.lock'))).to.be.false();
  });
});

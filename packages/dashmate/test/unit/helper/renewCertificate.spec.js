import fs from 'fs';
import ConfigFile from '../../../src/config/configFile/ConfigFile.js';
import ConfigFileJsonRepository from '../../../src/config/configFile/ConfigFileJsonRepository.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renewCertificate from '../../../src/helper/renewCertificate.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import obtainLetsEncryptCertificateTaskFactory from '../../../src/listr/tasks/ssl/letsencrypt/obtainLetsEncryptCertificateTaskFactory.js';
import obtainZeroSSLCertificateTaskFactory from '../../../src/listr/tasks/ssl/zerossl/obtainZeroSSLCertificateTaskFactory.js';
import getEnquirerMock from '../../../src/test/mock/getEnquirerMock.js';

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
          expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.true();
          config.set(
            'platform.gateway.ssl.providerConfigs.zerossl.id',
            'renewed-certificate-id-0000000000',
          );
        }),
      };
    });
    const writeConfigTemplates = this.sinon.stub().callsFake((config) => {
      expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.true();
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
    expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
  });

  it('should keep renewed state saved when service-file rendering fails', async function it() {
    const obtainCertificateTask = this.sinon.stub().callsFake((config) => ({
      run: this.sinon.stub().callsFake(async () => {
        config.set(
          'platform.gateway.ssl.providerConfigs.zerossl.id',
          'renewed-certificate-id-0000000000',
        );
      }),
    }));
    const writeConfigTemplates = this.sinon.stub().throws(new Error('template write failed'));

    await expect(renewCertificate({
      configName,
      provider: 'zerossl',
      expirationDays: 30,
      obtainCertificateTask,
      configFileRepository: repository,
      writeConfigTemplates,
    })).to.be.rejectedWith('template write failed');

    expect(repository.read().getConfig(configName)
      .get('platform.gateway.ssl.providerConfigs.zerossl.id'))
      .to.equal('renewed-certificate-id-0000000000');
    expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
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
    expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
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
    expect(fs.existsSync(homeDir.joinPath('.config.json.lock'))).to.be.false();
  });

  // The helper renews inside a container with no terminal, and its event loop
  // is kept alive by an interval that is never unref'd - so an unsettled prompt
  // there hangs forever rather than draining. It also holds the config lock,
  // whose mtime keeps being refreshed, so it never goes stale: renewal would
  // stop permanently and every command that changes configuration would fail on
  // a lock timeout until someone restarted the container.
  //
  // This drives the real obtain tasks through the context renewCertificate
  // actually builds, rather than asserting on the arguments it passes.
  describe('unattended renewal', () => {
    /**
     * @param {Object} tasks
     * @param {Object} enquirer
     * @return {Object}
     */
    function inject(tasks, enquirer) {
      // eslint-disable-next-line no-param-reassign
      tasks.options.injectWrapper = { enquirer };

      return tasks;
    }

    // noRetry is an operator control, not the interactivity guard. The helper
    // must be safe because it never opts in, so that dropping noRetry from this
    // call - or any other refactor - cannot make a background renewal prompt.
    it('should never opt the renewal context into prompting', async function it() {
      let context;

      await renewCertificate({
        configName,
        provider: 'zerossl',
        expirationDays: 2,
        obtainCertificateTask: () => ({
          run: async (ctx) => {
            context = ctx;
          },
        }),
        configFileRepository: repository,
        writeConfigTemplates: this.sinon.stub(),
      });

      expect(context.interactive).to.not.equal(true);
      expect(context.noRetry).to.be.true();
    });

    it('should construct no prompt on the Let\'s Encrypt renewal path', async function it() {
      repository.update((configFile) => {
        const config = configFile.getConfig(configName);
        config.set('platform.gateway.ssl.provider', 'letsencrypt');
        config.set('externalIp', '1.2.3.4');
      });

      const enquirer = getEnquirerMock(this.sinon, true);
      const missing = Object.assign(new Error('container not found'), { statusCode: 404 });
      const obtainLetsEncryptCertificateTask = obtainLetsEncryptCertificateTaskFactory(
        {
          getContainer: this.sinon.stub().rejects(missing),
          createContainer: this.sinon.stub().resolves({
            start: this.sinon.stub().resolves(),
            logs: this.sinon.stub().resolves(Buffer.from('Timeout during connect')),
            wait: this.sinon.stub().resolves({ StatusCode: 1 }),
          }),
        },
        this.sinon.stub().resolves(),
        { addContainer: this.sinon.stub() },
        homeDir,
        this.sinon.stub().resolves({ error: 'CERTIFICATE_NOT_FOUND', data: {} }),
        this.sinon.stub(),
        null,
        {},
      );

      await expect(renewCertificate({
        configName,
        provider: 'letsencrypt',
        expirationDays: 2,
        obtainCertificateTask: (config) => inject(
          obtainLetsEncryptCertificateTask(config),
          enquirer,
        ),
        configFileRepository: repository,
        writeConfigTemplates: this.sinon.stub(),
      })).to.be.rejected();

      expect(enquirer.prompt).to.not.have.been.called();
    });

    it('should construct no prompt on the ZeroSSL renewal path', async function it() {
      const enquirer = getEnquirerMock(this.sinon, true);
      const obtainZeroSSLCertificateTask = obtainZeroSSLCertificateTaskFactory(
        this.sinon.stub().resolves('csr'),
        this.sinon.stub().resolves({ privateKey: 'private', publicKey: 'public' }),
        this.sinon.stub().resolves({
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
        this.sinon.stub().rejects(new Error('domain control validation failed')),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        {
          setup: this.sinon.stub().resolves(),
          start: this.sinon.stub().resolves(),
          stop: this.sinon.stub().resolves(),
          destroy: this.sinon.stub().resolves(),
          waitForServerIsResponding: this.sinon.stub().resolves(true),
        },
        homeDir,
        this.sinon.stub(),
      );

      await expect(renewCertificate({
        configName,
        provider: 'zerossl',
        expirationDays: 2,
        obtainCertificateTask: (config, options) => {
          const tasks = inject(obtainZeroSSLCertificateTask(config, options), enquirer);

          const run = tasks.run.bind(tasks);
          tasks.run = (context) => run({
            ...context, force: true, externalIp: '1.2.3.4', apiKey: 'api-key',
          });

          return tasks;
        },
        configFileRepository: repository,
        writeConfigTemplates: this.sinon.stub(),
      })).to.be.rejected();

      expect(enquirer.prompt).to.not.have.been.called();
    });
  });
});

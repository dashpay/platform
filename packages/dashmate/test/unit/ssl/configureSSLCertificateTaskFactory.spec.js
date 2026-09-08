import { SSL_PROVIDERS } from '../../../src/constants.js';
import configureSSLCertificateTaskFactory
  from '../../../src/listr/tasks/setup/regular/configureSSLCertificateTaskFactory.js';

describe('configureSSLCertificateTaskFactory', () => {
  it('should checkpoint a ZeroSSL certificate created during setup', async function it() {
    const config = {
      set: this.sinon.stub(),
    };
    const configFile = {
      setConfig: this.sinon.stub(),
    };
    const configFileRepository = {
      write: this.sinon.stub(),
    };
    const obtainZeroSSLCertificateTask = this.sinon.stub()
      .callsFake((taskConfig, { onCertificateCreated }) => {
        onCertificateCreated();
        return 'obtain-task';
      });
    const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
      this.sinon.stub(),
      obtainZeroSSLCertificateTask,
      this.sinon.stub(),
      this.sinon.stub(),
      configFile,
      configFileRepository,
    );
    const context = {
      certificateProvider: SSL_PROVIDERS.ZEROSSL,
      config,
      nodeType: 'fullnode',
      preset: 'testnet',
    };
    const tasks = configureSSLCertificateTask();
    const providerTasks = await tasks.tasks[0].task(context, {
      prompt: this.sinon.stub(),
    });

    const result = await providerTasks.tasks[0].task(context, {
      prompt: this.sinon.stub().resolves('api-key'),
    });

    expect(result).to.equal('obtain-task');
    expect(configFile.setConfig).to.have.been.calledOnceWith(config);
    expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
  });

  // Setup is prompt-driven from end to end - it cannot run unattended - so it
  // is the one entry point that states interactivity outright rather than
  // detecting it.
  it('should mark the session as interactive for the obtain it starts', async function it() {
    const config = { set: this.sinon.stub() };
    const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      { setConfig: this.sinon.stub() },
      { write: this.sinon.stub() },
    );
    const context = {
      certificateProvider: SSL_PROVIDERS.SELF_SIGNED,
      config,
      nodeType: 'fullnode',
      preset: 'testnet',
    };

    await configureSSLCertificateTask().tasks[0].task(context, { prompt: this.sinon.stub() });

    expect(context.interactive).to.be.true();
  });

  // Nothing asks for a contact address any more. Let's Encrypt stopped sending
  // expiry notifications in 2025 and does not keep an address supplied through
  // ACME, so the question bought nothing and cost every new operator a step.
  it('should not ask for an email address', async function it() {
    const config = { set: this.sinon.stub() };
    const obtainLetsEncryptCertificateTask = this.sinon.stub().returns('obtain-task');
    const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      obtainLetsEncryptCertificateTask,
      { setConfig: this.sinon.stub() },
      { write: this.sinon.stub() },
    );
    const context = {
      certificateProvider: SSL_PROVIDERS.LETSENCRYPT,
      config,
      nodeType: 'fullnode',
      preset: 'testnet',
    };

    const providerTasks = await configureSSLCertificateTask().tasks[0]
      .task(context, { prompt: this.sinon.stub() });

    const prompt = this.sinon.stub();
    const result = await providerTasks.tasks[0].task(context, { prompt });

    expect(result).to.equal('obtain-task');
    expect(prompt).to.not.have.been.called();
    expect(config.set).to.not.have.been.calledWith(
      'platform.gateway.ssl.providerConfigs.letsencrypt.email',
    );
  });
});

import sinon from 'sinon';
import { SSL_PROVIDERS } from '../../../src/constants.js';
import NonInteractivePromptError from '../../../src/util/errors/NonInteractivePromptError.js';
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
      interactive: true,
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
    // Platform setup reports a later failure by whether this happened.
    expect(context.isZeroSslCertificateRecorded).to.be.true();
  });

  // Full setup states that it is interactive; Platform-only setup may not be.
  // The step itself must not decide, or an unattended run would wait on a
  // prompt nobody can answer.
  it('should leave interactivity to its caller', async function it() {
    const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      this.sinon.stub(),
      { setConfig: this.sinon.stub() },
      { write: this.sinon.stub() },
    );
    const prompt = this.sinon.stub();
    const context = { config: { set: this.sinon.stub() }, nodeType: 'fullnode', preset: 'testnet' };

    await expect(configureSSLCertificateTask().tasks[0].task(context, { prompt }))
      .to.be.rejectedWith(NonInteractivePromptError);
    expect(prompt).to.not.have.been.called();
    expect(context.interactive).to.be.undefined();
  });

  // update refuses a self-signed certificate on any masternode, so offering
  // one would set up a node that cannot be updated.
  describe('providers', () => {
    /**
     * @param {Object} context
     * @return {Promise<Object>} the provider prompt's options
     */
    async function providerPromptFor(context) {
      const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
        sinon.stub(),
        sinon.stub(),
        sinon.stub(),
        sinon.stub().returns('obtain-task'),
        { setConfig: sinon.stub() },
        { write: sinon.stub() },
      );
      const prompt = sinon.stub().resolves(SSL_PROVIDERS.LETSENCRYPT);

      await configureSSLCertificateTask().tasks[0].task(
        { config: { set: sinon.stub() }, interactive: true, ...context },
        { prompt },
      );

      return prompt.firstCall.args[0];
    }

    ['mainnet', 'testnet'].forEach((preset) => {
      it(`should not offer self-signed to a ${preset} masternode`, async () => {
        const { choices } = await providerPromptFor({ nodeType: 'masternode', preset });

        expect(choices.map(({ name }) => name)).to.deep.equal(['zerossl', 'letsencrypt', 'file']);
      });
    });

    it('should offer self-signed to a fullnode', async () => {
      const { choices } = await providerPromptFor({ nodeType: 'fullnode', preset: 'mainnet' });

      expect(choices.map(({ name }) => name)).to.include('self-signed');
    });

    it('should hold a provider given in advance to the same rule', async function it() {
      const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        this.sinon.stub(),
        { setConfig: this.sinon.stub() },
        { write: this.sinon.stub() },
      );
      const context = {
        config: { set: this.sinon.stub() },
        certificateProvider: SSL_PROVIDERS.SELF_SIGNED,
        nodeType: 'masternode',
        preset: 'testnet',
        interactive: false,
      };

      await expect(configureSSLCertificateTask().tasks[0].task(context, {}))
        .to.be.rejectedWith('cannot be used for a masternode');
    });
  });

  // The verbose renderer logs prompt redraws.
  it('should mask the ZeroSSL API key prompt', async function it() {
    const configureSSLCertificateTask = configureSSLCertificateTaskFactory(
      this.sinon.stub(),
      this.sinon.stub().returns('obtain-task'),
      this.sinon.stub(),
      this.sinon.stub(),
      { setConfig: this.sinon.stub() },
      { write: this.sinon.stub() },
    );
    const context = {
      certificateProvider: SSL_PROVIDERS.ZEROSSL,
      config: { set: this.sinon.stub() },
      nodeType: 'masternode',
      preset: 'testnet',
      interactive: true,
    };
    const providerTasks = await configureSSLCertificateTask().tasks[0]
      .task(context, { prompt: this.sinon.stub() });
    const prompt = this.sinon.stub().resolves('k'.repeat(32));

    await providerTasks.tasks[0].task(context, { prompt });

    expect(prompt.firstCall.args[0].type).to.equal('password');
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

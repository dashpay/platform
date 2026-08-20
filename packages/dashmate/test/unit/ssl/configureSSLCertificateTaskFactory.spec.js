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
});

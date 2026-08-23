import { Listr } from 'listr2';
import ObtainCommand from '../../../../src/commands/ssl/obtain.js';
import ServiceIsNotRunningError from '../../../../src/docker/errors/ServiceIsNotRunningError.js';

describe('SSL obtain command', () => {
  /**
   * @param {Object} sinon
   * @param {string} [provider]
   * @return {Object}
   */
  function obtainDependencies(sinon, provider = 'letsencrypt') {
    return {
      provider,
      config: {
        get: sinon.stub().returns(provider),
      },
      dockerCompose: {
        isServiceRunning: sinon.stub().resolves(true),
        execCommand: sinon.stub().resolves(),
      },
      obtainTask: sinon.stub().callsFake(() => new Listr([{ task: () => {} }])),
    };
  }

  /**
   * @param {Object} dependencies
   * @return {Promise}
   */
  function runObtain({
    provider, config, dockerCompose, obtainTask,
  }) {
    const noop = () => new Listr([]);

    return new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': true,
        'expiration-days': undefined,
        force: false,
        provider,
      },
      config,
      provider === 'zerossl' ? obtainTask : noop,
      provider === 'letsencrypt' ? obtainTask : noop,
      { write: () => {} },
      {},
      dockerCompose,
    );
  }

  // Envoy loads the certificate once at startup. Obtaining a certificate
  // without telling the gateway to reload leaves the operator with a command
  // that reports success while the node keeps serving the old certificate.
  it('should reload the gateway after obtaining a certificate', async function it() {
    const dependencies = obtainDependencies(this.sinon);

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand)
      .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
  });

  // ZeroSSL writes the certificate pair itself rather than going through
  // saveCertificateTask, so the reload cannot depend on anything that task
  // records.
  it('should reload the gateway after obtaining a ZeroSSL certificate', async function it() {
    const dependencies = obtainDependencies(this.sinon, 'zerossl');

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand)
      .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
  });

  // Nothing on disk reveals which certificate a running Envoy actually holds,
  // so an obtain that writes no new files still has to reload. Otherwise an
  // operator whose earlier reload failed can never recover by running the
  // command again.
  it('should reload the gateway when the certificate was already installed', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.obtainTask.callsFake(() => new Listr([
      {
        title: 'Certificate is up to date',
        skip: () => true,
        task: () => {},
      },
    ]));

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand)
      .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
  });

  // The certificate has already been obtained by the time the gateway is signalled, so a
  // gateway that is down must not turn the whole command into a failure - that would send the
  // operator back to a provider that may have nothing left to issue.
  it('should not fail when the gateway is not running', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.dockerCompose.execCommand
      .rejects(new ServiceIsNotRunningError('testnet', 'gateway'));

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand).to.have.been.calledOnce();
  });

  it('should still fail on a reload error that is not a stopped gateway', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.dockerCompose.execCommand.rejects(new Error('docker daemon is unreachable'));

    await expect(runObtain(dependencies)).to.be.rejected();
  });

  it('should checkpoint a newly created ZeroSSL certificate before a later failure', async function it() {
    const config = {
      get: this.sinon.stub().returns('zerossl'),
    };
    const configFile = {};
    const configFileRepository = {
      write: this.sinon.stub(),
    };
    const obtainZeroSSLCertificateTask = this.sinon.stub()
      .callsFake((taskConfig, { onCertificateCreated }) => new Listr([
        {
          task: async () => {
            onCertificateCreated();
            throw new Error('verification failed');
          },
        },
      ]));

    await expect(new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': true,
        'expiration-days': undefined,
        force: false,
        provider: 'zerossl',
      },
      config,
      obtainZeroSSLCertificateTask,
      this.sinon.stub(),
      configFileRepository,
      configFile,
    )).to.be.rejected();

    expect(obtainZeroSSLCertificateTask).to.have.been.calledOnce();
    expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
  });
});

import { Listr } from 'listr2';
import ObtainCommand from '../../../../src/commands/ssl/obtain.js';

describe('SSL obtain command', () => {
  /**
   * @param {Object} sinon
   * @param {boolean} certificateSaved
   * @return {Object}
   */
  function obtainDependencies(sinon, certificateSaved) {
    return {
      config: {
        get: sinon.stub().returns('letsencrypt'),
      },
      dockerCompose: {
        isServiceRunning: sinon.stub().resolves(true),
        execCommand: sinon.stub().resolves(),
      },
      obtainLetsEncryptCertificateTask: sinon.stub().callsFake(() => new Listr([
        {
          task: (ctx) => {
            ctx.certificateSaved = certificateSaved;
          },
        },
      ])),
    };
  }

  /**
   * @param {Object} dependencies
   * @return {Promise}
   */
  function runObtain({ config, dockerCompose, obtainLetsEncryptCertificateTask }) {
    return new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': true,
        'expiration-days': undefined,
        force: false,
        provider: 'letsencrypt',
      },
      config,
      () => new Listr([]),
      obtainLetsEncryptCertificateTask,
      { write: () => {} },
      {},
      dockerCompose,
    );
  }

  // Envoy loads the certificate once at startup. Obtaining a certificate
  // without telling the gateway to reload leaves the operator with a command
  // that reports success while the node keeps serving the old certificate.
  it('should reload the gateway after obtaining a certificate', async function it() {
    const dependencies = obtainDependencies(this.sinon, true);

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand)
      .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
  });

  it('should not reload the gateway when the certificate was already up to date', async function it() {
    const dependencies = obtainDependencies(this.sinon, undefined);

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand).to.have.not.been.called();
  });

  it('should not reload a gateway that is not running', async function it() {
    const dependencies = obtainDependencies(this.sinon, true);
    dependencies.dockerCompose.isServiceRunning.resolves(false);

    await runObtain(dependencies);

    expect(dependencies.dockerCompose.execCommand).to.have.not.been.called();
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

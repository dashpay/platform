import { Listr } from 'listr2';
import ObtainCommand from '../../../../src/commands/ssl/obtain.js';
import ServiceIsNotRunningError from '../../../../src/docker/errors/ServiceIsNotRunningError.js';

describe('SSL obtain command', () => {
  it('should reload the gateway so the new certificate is served', async function it() {
    // Writing the certificate files is not enough: the gateway keeps serving the previous
    // certificate until it is signalled, so an operator can run this command, see it succeed,
    // and find nothing changed on the wire.
    const config = {
      get: this.sinon.stub().callsFake((option) => (option === 'platform.enable' ? true : 'letsencrypt')),
    };
    const dockerCompose = {
      isServiceRunning: this.sinon.stub().resolves(true),
      execCommand: this.sinon.stub().resolves(),
    };

    await new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': true,
        'expiration-days': undefined,
        force: false,
        provider: 'letsencrypt',
      },
      config,
      this.sinon.stub(),
      this.sinon.stub().returns(new Listr([{ task: () => {} }])),
      { write: this.sinon.stub() },
      {},
      dockerCompose,
    );

    expect(dockerCompose.execCommand).to.have.been.calledOnceWith(config, 'gateway', 'kill -SIGHUP 1');
  });

  it('should not fail when the gateway stops before it can be signalled', async function it() {
    const config = {
      get: this.sinon.stub().callsFake((option) => (option === 'platform.enable' ? true : 'letsencrypt')),
    };
    const dockerCompose = {
      isServiceRunning: this.sinon.stub().resolves(true),
      execCommand: this.sinon.stub().rejects(new ServiceIsNotRunningError('testnet', 'gateway')),
    };

    await new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': true,
        'expiration-days': undefined,
        force: false,
        provider: 'letsencrypt',
      },
      config,
      this.sinon.stub(),
      this.sinon.stub().returns(new Listr([{ task: () => {} }])),
      { write: this.sinon.stub() },
      {},
      dockerCompose,
    );

    expect(dockerCompose.execCommand).to.have.been.calledOnce();
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
      { isServiceRunning: this.sinon.stub().resolves(false), execCommand: this.sinon.stub() },
    )).to.be.rejected();

    expect(obtainZeroSSLCertificateTask).to.have.been.calledOnce();
    expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
  });
});

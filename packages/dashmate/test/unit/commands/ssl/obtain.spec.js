import { Listr } from 'listr2';
import ObtainCommand from '../../../../src/commands/ssl/obtain.js';
import ServiceIsNotRunningError from '../../../../src/docker/errors/ServiceIsNotRunningError.js';

describe('SSL obtain command', () => {
  let stderr;

  beforeEach(function beforeEach() {
    stderr = '';
    this.sinon.stub(process.stderr, 'write').callsFake((chunk) => {
      stderr += chunk;
      return true;
    });
  });

  /**
   * @param {Object} sinon
   * @param {string} [provider]
   * @return {Object}
   */
  function obtainDependencies(sinon, provider = 'letsencrypt') {
    return {
      provider,
      observed: {},
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
   * Capture the context the obtain task is run with.
   *
   * @param {Object} dependencies
   * @return {Object}
   */
  function captureContext(sinon, dependencies) {
    const observed = {};

    // eslint-disable-next-line no-param-reassign
    dependencies.obtainTask = sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => Object.assign(observed, ctx),
    }]));

    return observed;
  }

  /**
   * @param {Object} dependencies
   * @return {Promise}
   */
  function runObtain({
    provider, config, dockerCompose, obtainTask, 'no-retry': noRetry = true,
  }) {
    const noop = () => new Listr([]);

    return new ObtainCommand().runWithDependencies(
      {},
      {
        verbose: false,
        'no-retry': noRetry,
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

  // The gate's own remediation tells the operator to run this command, and
  // §the permanence requirement is the whole reason the eight dark nodes went
  // dark. An operator who opens port 80 for one migration, runs this, succeeds
  // and closes the port again must not be able to do so silently.
  it('should state that port 80 has to stay open after obtaining', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => { ctx.certificateObtained = true; },
    }]));

    await runObtain(dependencies);

    expect(stderr).to.contain('LEAVE PORT 80 OPEN');
    expect(stderr).to.contain('survives a reboot');
  });

  // A certificate that was issued and then failed to install is still a
  // certificate this node now holds against its limits, and the operator who
  // opened port 80 for it is about to close it again. The failure must not
  // swallow the one thing that stops the node going dark in six days.
  it('should state permanence even when a later step fails', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => { ctx.certificateObtained = true; },
    }]));
    dependencies.dockerCompose.execCommand = this.sinon.stub()
      .rejects(new Error('reload failed'));

    await expect(runObtain(dependencies)).to.be.rejected();

    expect(stderr).to.contain('LEAVE PORT 80 OPEN');
  });

  // The permanence block belongs to the command, so a task that also carries it
  // would print it twice on the one path where both run.
  it('should state permanence once when the issued files never landed', async function it() {
    const dependencies = obtainDependencies(this.sinon);
    dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => {
        ctx.certificateObtained = true;

        throw new Error('Let\'s Encrypt issued a certificate, but dashmate could not find'
          + ' the file it should have written');
      },
    }]));

    await expect(runObtain(dependencies)).to.be.rejected();

    expect(stderr.split('LEAVE PORT 80 OPEN')).to.have.lengthOf(2);
  });

  // Nothing was issued, so there is nothing to warn about and a cron run stays
  // quiet.
  it('should stay silent when no new certificate was obtained', async function it() {
    const dependencies = obtainDependencies(this.sinon);

    await runObtain(dependencies);

    expect(stderr).to.not.contain('LEAVE PORT 80 OPEN');
  });

  // The retry loop lives in the shared obtain task, so `ssl obtain` gains it
  // too. Its --no-retry defaults to false, which would turn an obtain run from
  // cron into a hang if the flag were what decided whether to prompt.
  it('should not offer to prompt when run without a terminal', async function it() {
    // Set rather than inherited: run from a developer's terminal with CI
    // unset, the ambient streams are terminals and this stops proving
    // anything about the unattended path.
    const restore = { stdin: process.stdin.isTTY, stdout: process.stdout.isTTY };
    delete process.stdin.isTTY;
    delete process.stdout.isTTY;

    const dependencies = obtainDependencies(this.sinon);
    const context = captureContext(this.sinon, dependencies);

    try {
      await runObtain({ ...dependencies, 'no-retry': false });
    } finally {
      process.stdin.isTTY = restore.stdin;
      process.stdout.isTTY = restore.stdout;
    }

    expect(context.interactive).to.equal(false);
  });

  it('should offer to prompt an operator at a terminal', async function it() {
    // Built before the streams are touched, so nothing can throw between the
    // change and the restore that undoes it.
    const dependencies = obtainDependencies(this.sinon);
    const context = captureContext(this.sinon, dependencies);

    // A stream that is not a terminal has no isTTY property at all - not a
    // false one - so it is assigned rather than stubbed.
    const restore = { stdin: process.stdin.isTTY, stdout: process.stdout.isTTY, ci: process.env.CI };
    process.stdin.isTTY = true;
    process.stdout.isTTY = true;
    process.env.CI = '0';

    this.restoreStreams = () => {
      process.stdin.isTTY = restore.stdin;
      process.stdout.isTTY = restore.stdout;
      if (restore.ci === undefined) {
        delete process.env.CI;
      } else {
        process.env.CI = restore.ci;
      }
    };

    try {
      // The command's own default, not the helper's test-only one, since the
      // point is that interactivity does not come from this flag.
      await runObtain({ ...dependencies, 'no-retry': false });
    } finally {
      this.restoreStreams();
    }

    expect(context.interactive).to.equal(true);
  });
});

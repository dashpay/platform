import path from 'path';
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
      observed: {},
      config: {
        get: sinon.stub().returns(provider),
        getName: sinon.stub().returns('testnet'),
      },
      dockerCompose: {
        isServiceRunning: sinon.stub().resolves(true),
        execCommand: sinon.stub().resolves(),
      },
      obtainTask: sinon.stub().callsFake(() => new Listr([{ task: () => {} }])),
      homeDir: {
        joinPath: (...parts) => path.join('/home/dashmate', ...parts),
      },
      waitForGatewayToServeBundle: sinon.stub().resolves({ state: 'served', matchesOnDisk: true }),
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
    provider, config, dockerCompose, obtainTask, homeDir, waitForGatewayToServeBundle,
    'no-retry': noRetry = true,
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
      homeDir,
      waitForGatewayToServeBundle,
    );
  }

  // Envoy loads the certificate once at startup. Obtaining a certificate
  // without telling the gateway to reload leaves the operator with a command
  // that reports success while the node keeps serving the old certificate.
  // The notice belongs to the task that issues the certificate, so it renders
  // in the task list with everything else. The command writing it directly -
  // to either stream, from a finally or otherwise - is what this pins against.
  it('should not print the port 80 notice itself', async function it() {
    let written = '';
    ['stdout', 'stderr'].forEach((stream) => {
      this.sinon.stub(process[stream], 'write').callsFake((chunk) => {
        written += chunk;

        return true;
      });
    });

    const dependencies = obtainDependencies(this.sinon);
    dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
      task: (ctx) => { ctx.certificateObtained = true; },
    }]));

    await runObtain(dependencies);

    expect(written).to.not.contain('LEAVE PORT 80 OPEN');
  });
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

  // A bundle written as a new file is not the one the running gateway has
  // mounted, so a signal would only load the previous certificate again. The
  // command is printed on its own line after the task list, whose renderer
  // cuts a message off at the terminal width.
  describe('a bundle written as a new file', () => {
    let dependencies;
    let logged;

    beforeEach(function beforeEach() {
      logged = [];
      this.sinon.stub(console, 'log').callsFake((text) => logged.push(text));

      dependencies = obtainDependencies(this.sinon, 'zerossl');
      dependencies.config.getName = this.sinon.stub().returns('testnet');
      dependencies.config.get.withArgs('platform.quorumList.enabled').returns(false);
      dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
        task: (ctx) => { ctx.isGatewayRestartRequired = true; },
      }], { renderer: 'silent' }));
    });

    it('should name a gateway restart instead of reloading', async () => {
      await runObtain(dependencies);

      expect(dependencies.dockerCompose.execCommand).to.not.have.been.called();
      expect(logged.join('\n').split('\n')).to.include('  dashmate restart --config testnet --platform');
    });

    // stop --platform stops Core too when the quorum list is enabled, without
    // the DKG check a masternode gets before Core stops.
    it('should name a restart of the whole node when the quorum list is enabled', async () => {
      dependencies.config.get.withArgs('platform.quorumList.enabled').returns(true);

      await runObtain(dependencies);

      expect(logged.join('\n').split('\n')).to.include('  dashmate restart --config testnet');
    });
  });

  // A signal reloads the file the gateway has mounted, which need not be the
  // bundle on disk: one an earlier run replaced by a new file stays mounted
  // until the gateway restarts, and an obtain that finds the new bundle
  // already installed writes nothing. So what the gateway serves after the
  // signal is what decides.
  describe('what the gateway serves after the reload', () => {
    let dependencies;
    let logged;

    beforeEach(function beforeEach() {
      logged = [];
      this.sinon.stub(console, 'log').callsFake((text) => logged.push(text));

      dependencies = obtainDependencies(this.sinon, 'zerossl');
      dependencies.config.getName = this.sinon.stub().returns('testnet');
      dependencies.config.get.withArgs('platform.quorumList.enabled').returns(false);
    });

    it('should name a gateway restart, not a reload, when it still serves another certificate', async function it() {
      // The bundle was replaced by an earlier run, so this one writes nothing.
      dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
        title: 'Certificate is up to date',
        skip: () => true,
        task: () => {},
      }], { renderer: 'silent' }));
      dependencies.waitForGatewayToServeBundle.resolves({
        state: 'served',
        certificate: { fingerprint256: 'AA:AA' },
        onDisk: { fingerprint256: 'BB:BB' },
        matchesOnDisk: false,
      });

      await runObtain(dependencies);

      expect(dependencies.dockerCompose.execCommand)
        .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
      expect(dependencies.waitForGatewayToServeBundle).to.have.been.calledOnceWith(
        dependencies.config,
        path.join('/home/dashmate', 'testnet', 'platform', 'gateway', 'ssl', 'bundle.crt'),
      );

      const lines = logged.join('\n').split('\n');
      expect(lines.join('\n')).to.include('still serves a different certificate');
      expect(lines).to.include('  dashmate restart --config testnet --platform');
    });

    it('should report nothing more when it serves the bundle written in place', async function it() {
      dependencies.obtainTask = this.sinon.stub().callsFake(() => new Listr([{
        task: (ctx) => { ctx.isGatewayRestartRequired = false; },
      }], { renderer: 'silent' }));

      await runObtain(dependencies);

      expect(dependencies.dockerCompose.execCommand)
        .to.have.been.calledOnceWith(dependencies.config, 'gateway', 'kill -SIGHUP 1');
      expect(dependencies.waitForGatewayToServeBundle).to.have.been.calledOnce();
      expect(logged.join('\n')).to.not.include('dashmate restart')
        .and.not.include('dashmate doctor');
    });

    // The probe cannot tell an old certificate from no answer, so it neither
    // claims the certificate is served nor names a restart.
    it('should say the certificate could not be checked when the gateway does not answer', async function it() {
      dependencies.waitForGatewayToServeBundle.resolves({ state: 'unreachable', reason: 'ECONNREFUSED' });

      await runObtain(dependencies);

      const text = logged.join('\n');
      expect(text).to.include('ECONNREFUSED').and.include('dashmate doctor --config testnet');
      expect(text).to.not.include('dashmate restart');
    });

    // readCertificateBundle gives no bundle for a file it cannot read or
    // parse as well as for a missing one.
    it('should say the served certificate could not be compared with bundle.crt', async function it() {
      dependencies.waitForGatewayToServeBundle.resolves({
        state: 'served',
        certificate: { fingerprint256: 'AA:AA' },
        onDisk: null,
        matchesOnDisk: null,
      });

      await runObtain(dependencies);

      const text = logged.join('\n');
      expect(text).to.include('could not be compared with bundle.crt')
        .and.include('dashmate doctor --config testnet');
      expect(text).to.not.include('dashmate restart');
    });

    it('should not probe a gateway that is not running', async function it() {
      dependencies.dockerCompose.execCommand
        .rejects(new ServiceIsNotRunningError('testnet', 'gateway'));

      await runObtain(dependencies);

      expect(dependencies.waitForGatewayToServeBundle).to.not.have.been.called();
      expect(logged.join('\n')).to.not.include('dashmate restart')
        .and.not.include('dashmate doctor');
    });
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

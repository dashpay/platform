import { PassThrough } from 'node:stream';
import Docker from 'dockerode';
import UpdateCommand from '../../../src/commands/update.js';
import HomeDir from '../../../src/config/HomeDir.js';
import RenewalRecordRepository from '../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import updateNodeFactory from '../../../src/update/updateNodeFactory.js';
import CertificateUnresolvedError from '../../../src/ssl/errors/CertificateUnresolvedError.js';
import { CERTIFICATE_STATUS } from '../../../src/ssl/checkGatewayCertificateFactory.js';
import MuteOneLineError from '../../../src/oclif/errors/MuteOneLineError.js';

describe('Update command', () => {
  let config;
  let mockServicesList;
  let mockGetServicesList;
  let mockDocker;
  let mockDockerResponse;
  let dockerCompose;
  let homeDir;
  let stderr;
  let exitCode;

  /**
   * Docker answers a pull with a stream of newline separated JSON messages
   *
   * @param {Object[]|string[]} messages
   * @return {PassThrough}
   */
  function createPullStream(messages) {
    const stream = new PassThrough();

    messages.forEach((message) => {
      stream.write(typeof message === 'string' ? message : `${JSON.stringify(message)}\r\n`);
    });

    stream.end();

    return stream;
  }

  /**
   * @param {Object} verdict
   * @return {Object}
   */
  const passingVerdict = () => ({
    status: CERTIFICATE_STATUS.CHECKS_PASSED,
    reasons: [],
    warnings: [],
    skipped: [],
    provider: 'letsencrypt',
    installed: null,
    expiresInDays: 6,
  });

  /**
   * @param {Object} overrides
   * @return {Object}
   */
  const invalidVerdict = (overrides = {}) => ({
    ...passingVerdict(),
    status: CERTIFICATE_STATUS.INVALID,
    reasons: [{ code: 'EXPIRED', message: 'The installed certificate expired on 2026-05-01 - 111 days ago' }],
    ...overrides,
  });

  /**
   * @param {Object} options
   * @return {Promise}
   */
  function runUpdate({
    flags = {},
    updateNode,
    checkGatewayCertificate = () => passingVerdict(),
    gatewayCertificateTask = () => async () => {},
  } = {}) {
    return new UpdateCommand().runWithDependencies(
      {},
      {
        format: 'json',
        verbose: false,
        'skip-certificate-check': false,
        'non-interactive': false,
        ...flags,
      },
      mockDocker,
      config,
      updateNode ?? updateNodeFactory(mockGetServicesList, mockDocker),
      checkGatewayCertificate,
      gatewayCertificateTask,
      dockerCompose,
      new RenewalRecordRepository(homeDir),
    );
  }

  beforeEach(function it() {
    // The command reads the recorded renewal outcome from here, so it needs a
    // real directory rather than a stub - an absent record is a state the
    // guidance handles, and it is the one these tests are in.
    homeDir = HomeDir.createTemp();

    const getBaseConfig = getBaseConfigFactory(homeDir);

    config = getBaseConfig();
    config.set('network', 'mainnet');
    config.set('platform.enable', true);

    mockDockerResponse = { status: 'Status: Image is up to date for' };
    mockServicesList = [{ name: 'fake', image: 'fake', title: 'FAKE' }];

    mockGetServicesList = this.sinon.stub().callsFake(() => mockServicesList);
    mockDocker = {
      // The real modem is used on purpose: it owns the pull stream parsing
      modem: new Docker().modem,
      pull: this.sinon.stub()
        .callsFake((image, cb) => cb(false, createPullStream([mockDockerResponse]))),
    };
    dockerCompose = { isServiceRunning: this.sinon.stub().resolves(false) };

    stderr = '';
    this.sinon.stub(process.stderr, 'write').callsFake((chunk) => {
      stderr += chunk;
      return true;
    });

    exitCode = process.exitCode;
  });

  afterEach(() => {
    process.exitCode = exitCode;
  });

  it('should just update', async () => {
    await runUpdate();

    expect(mockGetServicesList).to.have.been.calledOnceWithExactly(config);
    expect(mockDocker.pull).to.have.been.calledOnceWith(mockServicesList[0].image);
  });

  it('should update other services if one of them fails', async function it() {
    mockServicesList = [{ name: 'fake', image: 'fake', title: 'FAKE' },
      { name: 'fake_docker_pull_error', image: 'fake_err_image', title: 'FAKE_ERROR' }];

    mockDocker = {
      modem: mockDocker.modem,
      pull: this.sinon.stub()
        .callsFake((image, cb) => (image === mockServicesList[1].image
          ? cb(new Error('pull access denied'), null)
          : cb(false, createPullStream([mockDockerResponse])))),
    };

    await runUpdate({ updateNode: updateNodeFactory(mockGetServicesList, mockDocker) });

    expect(mockGetServicesList).to.have.been.calledOnceWithExactly(config);
    expect(mockDocker.pull.firstCall.firstArg).to.equal(mockServicesList[0].image);
    expect(mockDocker.pull.secondCall.firstArg).to.equal(mockServicesList[1].image);
  });

  describe('the pull is never left unobserved', () => {
    // updateNode is async and calls getServiceList synchronously, and
    // docker.pull can throw synchronously inside its own executor, so this
    // promise really can reject - while the certificate task holds a prompt
    // open for minutes.
    it('should observe a pull that rejects immediately', async function it() {
      const rejection = new Error('service list is broken');
      let taskRan = false;

      await expect(runUpdate({
        updateNode: () => Promise.reject(rejection),
        gatewayCertificateTask: () => async () => {
          // An unobserved rejection would already have taken the process down.
          await new Promise((resolve) => { setImmediate(resolve); });
          taskRan = true;
        },
      })).to.be.rejectedWith(rejection);

      expect(taskRan).to.be.true();
    });

    // A pull that rejects fetched nothing at all. Reporting it and carrying on
    // hands a playbook running `update && start` a node whose images were never
    // fetched, with no exit code to catch - and set -e cannot see it.
    it('should fail the command when the pull rejects', async () => {
      const rejection = new Error('service list is broken');

      await expect(runUpdate({ updateNode: () => Promise.reject(rejection) }))
        .to.be.rejectedWith(rejection);
    });

    // The certificate guidance is still worth printing, but it is not what the
    // command failed on, and it must not replace the error that is.
    it('should still print the certificate guidance before failing on the pull', async () => {
      const rejection = new Error('service list is broken');
      const verdict = invalidVerdict();

      await expect(runUpdate({
        updateNode: () => Promise.reject(rejection),
        checkGatewayCertificate: () => verdict,
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = verdict;
          throw new CertificateUnresolvedError(verdict);
        },
      })).to.be.rejectedWith(rejection);

      expect(stderr).to.contain('is not valid');
    });

    // One Docker daemon being down fails the pull and the gateway reload at
    // once. Only one error can be thrown, and a total pull failure renders no
    // table of its own, so throwing the certificate error alone tells the
    // operator their certificate broke and never that no image arrived.
    it('should report the failed pull even when the certificate throws', async () => {
      const rejection = new Error('service list is broken');
      const unexpected = new Error('gateway reload failed');

      await expect(runUpdate({
        updateNode: () => Promise.reject(rejection),
        gatewayCertificateTask: () => async () => {
          throw unexpected;
        },
      })).to.be.rejectedWith(unexpected);

      expect(stderr).to.contain('service list is broken');
    });

    // The bypass flag and the verdict's list of checks that could not run are
    // two different things. Merging them under one name meant a machine reading
    // the line was told nothing was skipped on a node where the decisive
    // identity check never ran.
    it('should not let the bypass flag overwrite the skipped checks', async () => {
      const verdict = { ...invalidVerdict(), status: 'CHECKS_PASSED', reasons: [], skipped: ['IDENTITY'] };

      await runUpdate({
        flags: { format: 'json' },
        checkGatewayCertificate: () => verdict,
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = verdict;
          ctx.certificateSkipped = true;
        },
      });

      const line = stderr.split('\n').find((l) => l.trim().startsWith('{'));

      expect(JSON.parse(line).skipped).to.deep.equal(['IDENTITY']);
    });

    // The renderer can only tell the truth about a failed attempt if the
    // command actually tells it one happened, so the wiring is pinned here
    // rather than left to the renderer's own tests.
    it('should tell the guidance an obtain was attempted and failed', async () => {
      const verdict = invalidVerdict();

      // An unresolved certificate always exits non-zero, so the rejection is
      // the command working; the guidance it printed first is what is checked.
      await expect(runUpdate({
        checkGatewayCertificate: () => verdict,
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = verdict;
          ctx.certificateObtainError = new Error('lego exited 1');

          throw new CertificateUnresolvedError(verdict);
        },
      })).to.be.rejected();

      expect(stderr).to.contain('may have changed');
      expect(stderr).to.not.contain('Nothing broke just now');
    });

    // Defaulting an unknown state to stopped told an operator with a running
    // node the opposite of the truth, and offered them a command for it.
    it('should not report the node as stopped when Docker cannot be asked', async function it() {
      const verdict = invalidVerdict();

      dockerCompose.isServiceRunning = this.sinon.stub()
        .rejects(new Error('permission denied'));

      await expect(runUpdate({
        checkGatewayCertificate: () => verdict,
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = verdict;

          throw new CertificateUnresolvedError(verdict);
        },
      })).to.be.rejected();

      expect(stderr).to.not.contain('Your node is currently stopped');
    });

    // Individual images failing is not a rejection - updateNode resolves those
    // as error rows - but the exit code is the only thing that tells a caller
    // apart an update that fetched everything from one that fetched none of it.
    it('should exit non-zero without rejecting when individual pulls fail', async function it() {
      mockDocker = {
        modem: mockDocker.modem,
        pull: this.sinon.stub().callsFake((image, cb) => cb(new Error('registry down'), null)),
      };
      this.sinon.stub(console, 'log');

      await expect(runUpdate({ updateNode: updateNodeFactory(mockGetServicesList, mockDocker) }))
        .to.not.be.rejected();

      expect(process.exitCode).to.equal(1);
      expect(stderr).to.contain('registry down');
    });

    // exitOnError is false so the throw does not stop the list, and the
    // outer boundary covers the paths where task 2 cannot run at all.
    it('should report the pull exactly once', async function it() {
      const updateNode = this.sinon.stub().resolves([
        { name: 'fake', title: 'FAKE', updated: 'updated', image: 'fake' },
      ]);
      const log = this.sinon.stub(console, 'log');

      await runUpdate({ updateNode });

      expect(updateNode).to.have.been.calledOnce();
      expect(log).to.have.been.calledOnce();
    });

    // A lost config lock, a failed reload or a programming error is a real
    // failure. It must not be swallowed by exitOnError, and it must not be
    // reported as a certificate problem.
    it('should still report the pull when the certificate task throws unexpectedly', async () => {
      const unexpected = new Error('lost the configuration lock');

      await expect(runUpdate({
        gatewayCertificateTask: () => async () => {
          throw unexpected;
        },
      })).to.be.rejectedWith(unexpected);

      expect(mockDocker.pull).to.have.been.calledOnce();
    });
  });

  describe('an unresolved certificate', () => {
    /**
     * @param {Object} options
     * @return {Promise<Error>}
     */
    async function runUnresolved(options = {}) {
      const verdict = invalidVerdict();

      return runUpdate({
        checkGatewayCertificate: () => verdict,
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = verdict;
          throw new CertificateUnresolvedError(verdict);
        },
        ...options,
      }).catch((e) => e);
    }

    it('should pull images regardless of the verdict', async () => {
      await runUnresolved();

      expect(mockDocker.pull).to.have.been.calledOnce();
    });

    it('should exit non-zero through a muted error', async () => {
      const error = await runUnresolved();

      expect(error).to.be.an.instanceOf(MuteOneLineError);
      expect(error.getError()).to.be.an.instanceOf(CertificateUnresolvedError);
    });

    // The table has to be on screen before the failure is reported, so a failed
    // image row is never hidden behind an exit code.
    it('should render the table before the failure message', async function it() {
      const order = [];
      this.sinon.stub(console, 'log').callsFake(() => order.push('table'));
      process.stderr.write.callsFake((chunk) => {
        stderr += chunk;
        if (String(chunk).includes('is not valid')) {
          order.push('guidance');
        }
        return true;
      });

      await runUnresolved();

      expect(order).to.deep.equal(['table', 'guidance']);
    });

    // updateNode resolves a failed pull as an error row rather than rejecting,
    // so a registry outage plus a bad certificate would otherwise report that
    // patches were fetched when they were not.
    it('should not claim images were pulled when a pull failed', async function it() {
      mockServicesList = [{ name: 'a', image: 'a', title: 'A' }, { name: 'b', image: 'b', title: 'B' }];
      mockDocker = {
        modem: mockDocker.modem,
        pull: this.sinon.stub().callsFake((image, cb) => (image === 'b'
          ? cb(new Error('registry down'), null)
          : cb(false, createPullStream([mockDockerResponse])))),
      };
      this.sinon.stub(console, 'log');

      await runUnresolved({ updateNode: updateNodeFactory(mockGetServicesList, mockDocker) });

      expect(stderr).to.contain('1 of 2 failed');
    });
  });

  describe('scope', () => {
    ['local', 'devnet'].forEach((network) => {
      it(`should not check the certificate on ${network}`, async function it() {
        const innerTask = this.sinon.stub().resolves();
        const checkGatewayCertificate = this.sinon.stub().returns(passingVerdict());
        config.set('network', network);

        await runUpdate({
          checkGatewayCertificate,
          gatewayCertificateTask: () => innerTask,
        });

        // Nothing about the certificate is looked at, and the images are still
        // pulled - which is the whole of the rule for a network whose
        // certificate is self-signed by design.
        expect(innerTask).to.not.have.been.called();
        expect(checkGatewayCertificate).to.not.have.been.called();
        expect(mockDocker.pull).to.have.been.calledOnce();
      });
    });

    it('should not check the certificate when platform is disabled', async function it() {
      config.set('platform.enable', false);
      const task = this.sinon.stub().resolves();

      await runUpdate({ gatewayCertificateTask: () => task });

      expect(task).to.not.have.been.called();
      expect(mockDocker.pull).to.have.been.calledOnce();
    });
  });

  describe('flags', () => {
    it('should offer exactly the documented flags', () => {
      expect(Object.keys(UpdateCommand.flags).sort()).to.deep.equal([
        'config',
        'format',
        'non-interactive',
        'skip-certificate-check',
        'verbose',
      ]);
    });

    // The bypass suppresses enforcement, never the check, so a playbook that
    // carries it keeps surfacing the problem instead of muting it.
    it('should still run the check under --skip-certificate-check', async function it() {
      let observed;

      await runUpdate({
        flags: { 'skip-certificate-check': true },
        gatewayCertificateTask: (taskConfig, options) => {
          observed = options;
          return async (ctx) => {
            ctx.certificate = invalidVerdict();
            ctx.certificateSkipped = true;
          };
        },
      });

      expect(observed.skipCertificateCheck).to.be.true();
      expect(stderr).to.contain('certificate is not valid');
      expect(stderr).to.not.contain('status is INVALID');
    });

    it('should honour DASHMATE_SKIP_CERTIFICATE_CHECK', async function it() {
      this.sinon.stub(process, 'env').value({ ...process.env, DASHMATE_SKIP_CERTIFICATE_CHECK: '1' });
      let observed;

      await runUpdate({
        gatewayCertificateTask: (taskConfig, options) => {
          observed = options;
          return async () => {};
        },
      });

      expect(observed.skipCertificateCheck).to.be.true();
    });

    it('should never prompt under --non-interactive', async function it() {
      let observed;

      await runUpdate({
        flags: { 'non-interactive': true },
        gatewayCertificateTask: (taskConfig, options) => {
          observed = options;
          return async () => {};
        },
      });

      expect(observed.interactive).to.be.false();
    });
  });

  describe('machine output', () => {
    // Under JSON output stdout carries exactly one parseable array, so nothing
    // about the certificate may be written there.
    it('should keep the certificate diagnostics off stdout', async function it() {
      const log = this.sinon.stub(console, 'log');

      await runUpdate({
        checkGatewayCertificate: () => invalidVerdict(),
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = invalidVerdict();
        },
      });

      expect(log).to.have.been.calledOnce();
      expect(() => JSON.parse(log.firstCall.firstArg)).to.not.throw();
      expect(stderr).to.contain('"status":"INVALID"');
    });

    // reasons and warnings are ordered arrays because several can be true at
    // once, and collapsing them to one value reintroduces a precedence nobody
    // defined.
    it('should emit reasons and warnings as arrays alongside the pull result', async () => {
      await runUpdate({
        gatewayCertificateTask: () => async (ctx) => {
          ctx.certificate = {
            ...invalidVerdict(),
            warnings: [{ code: 'PROVIDER_MISMATCH', message: 'x' }],
          };
        },
      }).catch(() => {});

      const line = stderr.split('\n').find((entry) => entry.startsWith('{'));
      const diagnostics = JSON.parse(line);

      expect(diagnostics.reasons).to.deep.equal(['EXPIRED']);
      expect(diagnostics.warnings).to.deep.equal(['PROVIDER_MISMATCH']);
      expect(diagnostics.pull).to.deep.equal({ ok: true, failed: 0, total: 1 });
      expect(diagnostics.status).to.not.equal('VALID');
    });
  });

  describe('failed image pulls', () => {
    const rateLimitMessage = 'toomanyrequests: You have reached your pull rate limit';

    // Docker answers a pull request with 200 and reports registry and disk
    // failures as a message inside the progress stream, so a stream that
    // completes does not mean the image arrived.
    it('should exit non-zero and print the reason when the registry rate limits the pull', async function it() {
      mockDockerResponse = {
        errorDetail: { message: rateLimitMessage },
        error: rateLimitMessage,
      };

      this.sinon.stub(console, 'log');

      await runUpdate();

      expect(process.exitCode).to.equal(1);
      expect(stderr).to.contain(rateLimitMessage);
    });

    // The previous parser split every chunk on CRLF and parsed each fragment,
    // so an ordinary TCP boundary threw inside the stream handler.
    it('should report the reason when Docker splits a message across chunks', async function it() {
      const line = JSON.stringify({
        errorDetail: { message: rateLimitMessage },
        error: rateLimitMessage,
      });

      mockDocker.pull = this.sinon.stub().callsFake((image, cb) => cb(
        false,
        createPullStream([line.slice(0, 20), `${line.slice(20)}\n`]),
      ));

      const [updateInfo] = await updateNodeFactory(mockGetServicesList, mockDocker)(config);

      expect(updateInfo.updated).to.equal('error');
      expect(updateInfo.error).to.equal(rateLimitMessage);
    });

    // Docker separates messages with a line feed and can deliver several of
    // them in a single chunk.
    it('should report the reason when Docker separates messages with a line feed', async function it() {
      const chunk = [
        JSON.stringify({ status: 'Pulling from dashpay/drive' }),
        JSON.stringify({ errorDetail: { message: rateLimitMessage }, error: rateLimitMessage }),
        '',
      ].join('\n');

      mockDocker.pull = this.sinon.stub()
        .callsFake((image, cb) => cb(false, createPullStream([chunk])));

      const [updateInfo] = await updateNodeFactory(mockGetServicesList, mockDocker)(config);

      expect(updateInfo.updated).to.equal('error');
      expect(updateInfo.error).to.equal(rateLimitMessage);
    });

    // A pull can fail after the layers arrive - a full disk is the usual
    // reason - and the last status line would otherwise report a success.
    it('should exit non-zero when the pull fails after the image was downloaded', async function it() {
      const diskFullMessage = 'write /var/lib/docker/tmp: no space left on device';

      mockDocker.pull = this.sinon.stub().callsFake((image, cb) => cb(
        false,
        createPullStream([
          { status: 'Status: Downloaded newer image for fake' },
          { errorDetail: { message: diskFullMessage }, error: diskFullMessage },
        ]),
      ));

      this.sinon.stub(console, 'log');

      await runUpdate();

      expect(process.exitCode).to.equal(1);
      expect(stderr).to.contain(diskFullMessage);
    });

    it('should keep the JSON output parseable and carry the reason', async function it() {
      mockDocker.pull = this.sinon.stub()
        .callsFake((image, cb) => cb(new Error('no space left on device'), null));

      const consoleLog = this.sinon.stub(console, 'log');

      await runUpdate();

      expect(consoleLog).to.have.been.calledOnce();

      const output = JSON.parse(consoleLog.firstCall.firstArg);

      expect(output).to.have.lengthOf(1);
      expect(output[0].updated).to.equal('error');
      expect(output[0].error).to.include('no space left on device');
    });

    it('should print the reason to stderr and keep the table intact in plain output', async function it() {
      const diskFullMessage = 'write /var/lib/docker/tmp: no space left on device';

      mockServicesList = [
        { name: 'core', image: 'dashpay/dashd:23', title: 'Core' },
        { name: 'drive_abci', image: 'dashpay/drive:4', title: 'Drive ABCI' },
      ];

      mockDocker.pull = this.sinon.stub().callsFake((image, cb) => (image === 'dashpay/drive:4'
        ? cb(new Error(diskFullMessage), null)
        : cb(false, createPullStream([mockDockerResponse]))));

      const consoleLog = this.sinon.stub(console, 'log');

      await runUpdate({ flags: { format: 'plain' } });

      // Plain output renders the task list through console.log as well,
      // so the table is one of several calls
      const stdout = consoleLog.args.flat().join('\n');

      expect(stdout).to.include('Drive ABCI');
      expect(stdout).to.include('error');
      // The table has no column for it, so the reason is only reported on stderr
      expect(stdout).to.not.include(diskFullMessage);

      expect(stderr).to.contain('Failed to update 1 of 2 images');
      expect(stderr).to.contain(`Drive ABCI (dashpay/drive:4): ${diskFullMessage}`);
      expect(process.exitCode).to.equal(1);
    });

    // The registry chooses this text. Printed as it arrives, an escape sequence
    // in it can rewrite lines already on screen, hide what follows or address
    // the terminal itself, and an unbounded message pushes the rest of the
    // output out of the operator's scrollback.
    it('should not let the registry write escape sequences to the terminal', async function it() {
      const hostile = `\u001b[2J\u001b[1;1HImage is up to date\u0007${'A'.repeat(5000)}`;

      mockDockerResponse = { errorDetail: { message: hostile }, error: hostile };

      this.sinon.stub(console, 'log');

      await runUpdate();

      expect(stderr).to.not.contain('\u001b');
      expect(stderr).to.not.contain('\u0007');
      expect(stderr).to.contain('Image is up to date');
      expect(stderr.length).to.be.below(2000);
    });

    it('should show images built from local sources as built locally', async function it() {
      mockServicesList = [
        { name: 'fake', image: 'fake', title: 'FAKE' },
        {
          name: 'drive_abci', image: 'drive:local', title: 'Drive ABCI', isBuiltLocally: true,
        },
      ];

      const consoleLog = this.sinon.stub(console, 'log');

      await runUpdate();

      expect(mockDocker.pull).to.have.been.calledOnceWith('fake');

      const output = JSON.parse(consoleLog.firstCall.firstArg);

      expect(output).to.have.lengthOf(2);
      expect(output[1]).to.deep.equal({
        name: 'drive_abci',
        title: 'Drive ABCI',
        image: 'drive:local',
        updated: 'built locally',
      });

      expect(process.exitCode).to.equal(0);
    });
  });

  // A prompt that leaks past the interactivity guard neither throws nor
  // settles - the event loop drains and the process exits 0 with nothing done.
  // The entry-time exit code is the only thing that turns that into a failure.
  it('should fail closed until the run resolves', async () => {
    let duringRun;

    await runUpdate({
      gatewayCertificateTask: () => async () => {
        duringRun = process.exitCode;
      },
    });

    expect(duringRun).to.equal(1);
    expect(process.exitCode).to.equal(0);
  });

  it('should not clear the exit code when the certificate is unresolved', async () => {
    const verdict = invalidVerdict();

    await runUpdate({
      gatewayCertificateTask: () => async () => {
        throw new CertificateUnresolvedError(verdict);
      },
    }).catch(() => {});

    expect(process.exitCode).to.equal(1);
  });
});

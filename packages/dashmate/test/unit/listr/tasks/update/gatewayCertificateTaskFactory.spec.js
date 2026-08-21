import fs from 'fs';
import path from 'path';
import { Listr } from 'listr2';
import HomeDir from '../../../../../src/config/HomeDir.js';
import getBaseConfigFactory from '../../../../../configs/defaults/getBaseConfigFactory.js';
import gatewayCertificateTaskFactory from '../../../../../src/listr/tasks/update/gatewayCertificateTaskFactory.js';
import CertificateUnresolvedError from '../../../../../src/ssl/errors/CertificateUnresolvedError.js';
import {
  CERTIFICATE_REASONS,
  CERTIFICATE_STATUS,
} from '../../../../../src/ssl/checkGatewayCertificateFactory.js';
import getEnquirerMock from '../../../../../src/test/mock/getEnquirerMock.js';
import ServiceIsNotRunningError from '../../../../../src/docker/errors/ServiceIsNotRunningError.js';

describe('gatewayCertificateTaskFactory', () => {
  let homeDir;
  let config;
  let configFile;
  let configFileRepository;
  let writeConfigTemplates;
  let dockerCompose;
  let obtainLetsEncryptCertificateTask;
  let installCertificateFilesTask;
  let enquirer;

  /**
   * @param {Object} [overrides]
   * @return {Object}
   */
  const verdict = (overrides = {}) => ({
    status: CERTIFICATE_STATUS.CHECKS_PASSED,
    reasons: [],
    warnings: [],
    skipped: [],
    provider: config.get('platform.gateway.ssl.provider'),
    installed: { validTo: new Date(Date.now() + 6 * 864e5) },
    expiresInDays: 6,
    ...overrides,
  });

  /**
   * @param {Object} overrides
   * @return {Object}
   */
  const invalid = (code = CERTIFICATE_REASONS.EXPIRED, overrides = {}) => verdict({
    status: CERTIFICATE_STATUS.INVALID,
    reasons: [{ code, message: `certificate problem: ${code}` }],
    ...overrides,
  });

  /**
   * Run the task the way the update command runs it, with a stubbed enquirer so
   * a prompt can be answered - or its absence asserted.
   *
   * @param {Object} options
   * @return {Promise<Object>} the listr context
   */
  async function run({
    checkGatewayCertificate,
    interactive = true,
    skipCertificateCheck = false,
    answers = [],
  }) {
    enquirer = getEnquirerMock(this.sinon, ...answers);

    const gatewayCertificateTask = gatewayCertificateTaskFactory(
      checkGatewayCertificate,
      obtainLetsEncryptCertificateTask,
      installCertificateFilesTask,
      configFileRepository,
      configFile,
      writeConfigTemplates,
      dockerCompose,
    );

    const context = {};
    const tasks = new Listr([{
      title: 'Gateway certificate',
      task: gatewayCertificateTask(config, { interactive, skipCertificateCheck }),
    }], { renderer: 'silent', exitOnError: false });

    tasks.options.injectWrapper = { enquirer };

    await tasks.run(context);

    return {
      context,
      errors: (tasks.err ?? []).map((e) => e?.error ?? e),
      state: tasks.tasks[0].state,
    };
  }

  beforeEach(function it() {
    homeDir = HomeDir.createTemp();
    config = getBaseConfigFactory(homeDir)();
    config.set('network', 'mainnet');
    config.set('externalIp', '1.2.3.4');
    config.set('platform.gateway.ssl.provider', 'zerossl');
    config.markAsSaved();

    configFile = { getConfig: () => config };
    configFileRepository = {
      isExclusive: this.sinon.stub().returns(true),
      write: this.sinon.stub(),
    };
    writeConfigTemplates = this.sinon.stub();
    dockerCompose = { execCommand: this.sinon.stub().resolves() };
    obtainLetsEncryptCertificateTask = this.sinon.stub()
      .callsFake(() => new Listr([{ task: () => {} }], { renderer: 'silent' }));
    installCertificateFilesTask = this.sinon.stub()
      .callsFake(() => new Listr([{ task: () => {} }], { renderer: 'silent' }));
  });

  afterEach(() => homeDir.remove());

  // A ZeroSSL node reaches WARN through any warning at all - the certificate
  // running out inside a day, an unmanaged pair, a provider that disagrees with
  // the configuration. The switch offer is the whole point of noticing ZeroSSL,
  // and it is needed most in exactly those states, so it must not be reachable
  // only from a completely clean verdict.
  describe('a ZeroSSL node that also has a warning', () => {
    const expiringSoon = () => verdict({
      status: CERTIFICATE_STATUS.WARN,
      warnings: [{ code: CERTIFICATE_REASONS.EXPIRING_SOON, message: 'expires tomorrow' }],
      expiresInDays: 0.5,
    });

    it('should still offer the switch when expiry is hours away', async function it() {
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => expiringSoon(),
      });

      expect(errors).to.be.empty();
      expect(enquirer.prompt).to.have.been.called();
      expect(enquirer.options[0].message).to.contain("Switch to Let's Encrypt");
    });

    it('should still explain the ZeroSSL wall to a machine', async function it() {
      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => expiringSoon(),
        interactive: false,
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings.join('\n')).to.contain('three certificates in total');
    });

    // The reason it reached WARN is a fact about this node in its own right and
    // has to survive: it is what tells the operator the certificate is nearly
    // out, which is why the switch is urgent at all.
    it('should keep the warning that made it WARN', async function it() {
      const { context } = await run.call(this, {
        checkGatewayCertificate: () => expiringSoon(),
        interactive: false,
      });

      expect(context.certificateWarnings.join('\n')).to.contain('expires tomorrow');
    });

    it('should still report a non-ZeroSSL warning on its own', async function it() {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict({
          status: CERTIFICATE_STATUS.WARN,
          provider: 'letsencrypt',
          warnings: [{ code: CERTIFICATE_REASONS.EXPIRING_SOON, message: 'expires tomorrow' }],
        }),
        interactive: false,
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings).to.deep.equal(['expires tomorrow']);
      expect(enquirer.prompt).to.not.have.been.called();
    });
  });

  // The branch is entered on the configured provider, which is not evidence of
  // who issued the leaf that is actually installed. Under a provider mismatch
  // the certificate on disk can come from anywhere, so nothing here may assert
  // what it is, and nothing may claim it passed checks it only warned on.
  describe('a ZeroSSL node whose installed leaf is not ZeroSSL', () => {
    const warned = (code) => verdict({
      status: CERTIFICATE_STATUS.WARN,
      warnings: [{ code, message: `warning: ${code}` }],
    });

    [
      CERTIFICATE_REASONS.PROVIDER_MISMATCH,
      CERTIFICATE_REASONS.SSL_UNMANAGED,
      CERTIFICATE_REASONS.SELF_SIGNED,
    ].forEach((code) => {
      it(`should not call the installed leaf a ZeroSSL certificate on ${code}`, async function it() {
        const { context, errors } = await run.call(this, {
          checkGatewayCertificate: () => warned(code),
          interactive: false,
        });

        expect(errors).to.be.empty();

        const warnings = context.certificateWarnings.join('\n');

        expect(warnings).to.contain('three certificates in total');
        expect(warnings).to.not.contain("This node's ZeroSSL certificate");
        expect(warnings).to.contain(`warning: ${code}`);
      });

      it(`should still offer the switch on ${code}`, async function it() {
        const { errors } = await run.call(this, {
          checkGatewayCertificate: () => warned(code),
        });

        expect(errors).to.be.empty();
        expect(enquirer.prompt).to.have.been.called();
      });
    });

    it('should not tell a warned node its certificate passed its checks', async function it() {
      await run.call(this, {
        checkGatewayCertificate: () => warned(CERTIFICATE_REASONS.PROVIDER_MISMATCH),
      });

      expect(enquirer.options[0].header).to.not.contain('passed its checks');
    });

    it('should still say the certificate passed when the verdict is clean', async function it() {
      await run.call(this, { checkGatewayCertificate: () => verdict() });

      expect(enquirer.options[0].header).to.contain('passed its checks');
    });
  });

  describe('nothing blocks on a certificate that passed', () => {
    it('should say nothing at all for a provider that is working', async function it() {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict(),
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings).to.be.undefined();
      expect(enquirer.prompt).to.not.have.been.called();
    });

    // A free ZeroSSL account allows three certificates in total and no REST
    // access, so renewal stops permanently and the operator has no way to find
    // that out before it happens. This is the only passing case that speaks.
    it('should always warn about a ZeroSSL certificate, even to a machine', async function it() {
      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict(),
        interactive: false,
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings.join('\n')).to.contain('expires in 6 days');
      expect(enquirer.prompt).to.not.have.been.called();
    });

    it('should preselect the switch only when time is short', async function it() {
      await run.call(this, {
        checkGatewayCertificate: () => verdict({ expiresInDays: 40 }),
        answers: [false],
      });
      expect(enquirer.options[0].initial).to.be.false();

      await run.call(this, {
        checkGatewayCertificate: () => verdict({ expiresInDays: 5 }),
        answers: [false],
      });
      expect(enquirer.options[0].initial).to.be.true();
    });

    // The courtesy offer is only made when the installed certificate passed the
    // checks and stays in place. Telling that operator declining leaves clients
    // unable to connect contradicts what the same run just established.
    it('should not tell an operator whose certificate passed that clients cannot connect', async function it() {
      await run.call(this, {
        checkGatewayCertificate: () => verdict(),
        answers: [false],
      });

      const { header } = enquirer.options[0];

      expect(header).to.not.match(/clients (are|were|could not|cannot|unable)/i);
      expect(header).to.contain('nothing changes');
    });

    // The same wording is correct on the failing path, where the certificate
    // really did not pass, so the offer has to say different things.
    it('should say what declining costs when the certificate did not pass', async function it() {
      await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        answers: [false],
      });

      // What declining costs, in terms of what was actually established: the
      // installed pair stays as it is and still fails the checks. Not what a
      // client would do with it - the checks read files, never the wire, and
      // never the chain to a public root.
      const { header } = enquirer.options[0];

      expect(header).to.contain('still failing the checks');
      expect(header).to.not.match(/client (will|would|does not|will not)? ?(accept|reject)/i);
      expect(header).to.not.match(/clients (are|were|could not|cannot|unable)/i);
    });

    it('should exit cleanly when the courtesy switch is declined', async function it() {
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict(),
        answers: [false],
      });

      expect(errors).to.be.empty();
      expect(obtainLetsEncryptCertificateTask).to.not.have.been.called();
    });
  });

  describe('the courtesy migration is judged by what it left behind', () => {
    // Nothing was blocking before this ran. An obtain that failed before
    // touching the gateway files leaves the node exactly as it was, so it is a
    // warning - reporting an error would tell an operator to act on a node that
    // is fine.
    it('should warn when a failed switch changed nothing', async function it() {
      obtainLetsEncryptCertificateTask.callsFake(() => ({
        run: async () => {
          throw new Error('port 80 is closed');
        },
      }));

      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict(),
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings.join('\n')).to.contain('did not complete');
      expect(context.certificateWarnings.join('\n')).to.contain('untouched');
    });

    // The node is no longer on ZeroSSL, so repeating its expiry alongside the
    // success message would contradict it.
    it('should drop the ZeroSSL warning once the switch succeeds', async function it() {
      let checked = 0;
      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? verdict() : verdict({ provider: 'letsencrypt' });
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings).to.be.undefined();
      expect(context.certificateSuccess).to.contain('LEAVE PORT 80 OPEN');
    });

    // Nothing was touched, so nothing regressed - and a certificate that
    // crossed the expiring-soon boundary during a multi-minute failed obtain
    // re-checks as a warning, not a pass. Demanding an exact pass there fails
    // the run on a node that is exactly as it was.
    it('should stay benign when a failed switch leaves only a warning behind', async function it() {
      obtainLetsEncryptCertificateTask.callsFake(() => ({
        run: async () => {
          throw new Error('port 80 is closed');
        },
      }));

      let checked = 0;
      const { errors, context } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? verdict() : verdict({
            status: CERTIFICATE_STATUS.WARN,
            warnings: [{ code: CERTIFICATE_REASONS.EXPIRING_SOON, message: 'expires tomorrow' }],
          });
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(context.certificateWarnings.join('\n')).to.contain('did not complete');
    });

    // saveCertificateTask writes the bundle and the key as two separate
    // in-place writes, so a failure between them can replace a working pair
    // with a mismatched one. Promising exit 0 here would tell an operator their
    // node is fine at the moment it stopped serving TLS.
    it('should fail when a failed switch damaged the installed pair', async function it() {
      obtainLetsEncryptCertificateTask.callsFake(() => ({
        run: async () => {
          throw new Error('no space left on device');
        },
      }));

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? verdict() : invalid(CERTIFICATE_REASONS.KEY_MISMATCH);
        },
        answers: [true],
      });

      expect(errors[0]).to.be.an.instanceOf(CertificateUnresolvedError);
    });
  });

  describe('the provider is persisted only after a certificate exists', () => {
    // Writing the provider first and then failing the obtain converts a node
    // that was working with an expiring certificate into one that is broken:
    // configuration would name an authority it has no account with, and the
    // helper's watcher would reschedule renewal against it within a minute,
    // forever.
    it('should not persist anything when the obtain fails', async function it() {
      obtainLetsEncryptCertificateTask.callsFake(() => ({
        run: async () => {
          throw new Error('lego failed');
        },
      }));

      await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        answers: [true],
      });

      expect(configFileRepository.write).to.not.have.been.called();
      expect(writeConfigTemplates).to.not.have.been.called();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('zerossl');
    });

    it('should persist after the obtain succeeds', async function it() {
      let checked = 0;
      await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
      expect(writeConfigTemplates).to.have.been.calledOnceWith(config);
    });

    // Issuance takes minutes, long enough for the lease to be lost and another
    // command to save and render newer state. The state this leaves behind is
    // the interrupted switch, which the next run detects and converges on.
    it('should refuse to write once the lock is gone', async function it() {
      configFileRepository.isExclusive.returns(false);

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(errors[0].message).to.contain('Lost the configuration lock');
      expect(configFileRepository.write).to.not.have.been.called();
    });
  });

  describe('an interrupted switch converges', () => {
    it('should finish the persistence without obtaining anything', async function it() {
      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1
            ? invalid(CERTIFICATE_REASONS.SWITCH_INCOMPLETE)
            : verdict({ provider: 'letsencrypt' });
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(obtainLetsEncryptCertificateTask).to.not.have.been.called();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('letsencrypt');
      expect(configFileRepository.write).to.have.been.calledOnce();
    });

    // The pair being the one lego produced says nothing about whether it is
    // still valid, so this state can carry an expired certificate too. Saving
    // the setting there fixes the helper's target and leaves the node dark -
    // and telling the operator it is fixed is worse than saying nothing.
    it('should not claim a dead certificate is fixed by saving the setting', async function it() {
      const dead = invalid(CERTIFICATE_REASONS.SWITCH_INCOMPLETE, {
        reasons: [
          { code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE, message: 'switch interrupted' },
          { code: CERTIFICATE_REASONS.EXPIRED, message: 'expired 158 days ago' },
        ],
      });

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? dead : verdict();
        },
        answers: [true],
      });

      // A certificate is genuinely needed here, so the operator is asked for
      // one rather than told the setting is all that was missing.
      expect(obtainLetsEncryptCertificateTask).to.have.been.calledOnce();
      expect(errors).to.be.empty();
      expect(enquirer.options[0].message).to.not.contain('Finish the interrupted switch');
    });

    // Persisting can fail, the reload can fail, and the installed pair can have
    // expired between the check and the write. Success is what the re-check
    // says, not what the branch assumed.
    it('should fail when the certificate is still unusable after completing', async function it() {
      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1
            ? invalid(CERTIFICATE_REASONS.SWITCH_INCOMPLETE)
            : invalid(CERTIFICATE_REASONS.KEY_MISMATCH);
        },
        answers: [true],
      });

      expect(errors[0]).to.be.an.instanceOf(CertificateUnresolvedError);
    });

    it('should block when the operator declines to finish it', async function it() {
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => invalid(CERTIFICATE_REASONS.SWITCH_INCOMPLETE),
        answers: [false],
      });

      expect(errors[0]).to.be.an.instanceOf(CertificateUnresolvedError);
      expect(config.get('platform.gateway.ssl.provider')).to.equal('zerossl');
    });
  });

  describe('an operator with their own certificate', () => {
    it('should offer to install replacement files before suggesting a new authority', async function it() {
      config.set('platform.gateway.ssl.provider', 'file');

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(installCertificateFilesTask).to.have.been.calledOnce();
      expect(obtainLetsEncryptCertificateTask).to.not.have.been.called();
      expect(enquirer.options[0].message).to.contain('Install new certificate files');
    });

    // The gateway listener is branched on the provider: self-signed renders a
    // tls_inspector plus a raw_buffer filter chain, so the port keeps accepting
    // plaintext. Installing a real certificate without recording that the
    // provider changed leaves that chain in place on a node whose operator has
    // just done exactly the right thing.
    it('should record the new provider after installing operator files', async function it() {
      config.set('platform.gateway.ssl.provider', 'self-signed');

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid(CERTIFICATE_REASONS.SELF_SIGNED) : verdict();
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
      expect(config.get('platform.gateway.ssl.provider')).to.equal('file');
      expect(config.get('platform.gateway.ssl.enabled')).to.be.true();
    });

    // The gate is the only caller that persists mid-run, because update carries
    // on into a multi-minute pull afterwards and the end-of-run save is skipped
    // whenever the run later throws.
    it('should persist and re-render immediately after installing files', async function it() {
      config.set('platform.gateway.ssl.provider', 'self-signed');

      let checked = 0;
      await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid(CERTIFICATE_REASONS.SELF_SIGNED) : verdict();
        },
        answers: [true],
      });

      expect(configFileRepository.write).to.have.been.calledOnceWith(configFile);
      expect(writeConfigTemplates).to.have.been.calledOnceWith(config);
    });

    // The verdict after the install can still carry warnings - a provider that
    // disagrees with the new certificate's issuer, say - and the command prints
    // only what the task collects.
    it('should carry warnings from the installed certificate', async function it() {
      config.set('platform.gateway.ssl.provider', 'file');

      let checked = 0;
      const { context } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict({
            status: CERTIFICATE_STATUS.WARN,
            warnings: [{ code: CERTIFICATE_REASONS.PROVIDER_MISMATCH, message: 'issuer disagrees' }],
          });
        },
        answers: [true],
      });

      expect(context.certificateWarnings).to.deep.equal(['issuer disagrees']);
    });

    // Changing the authority on a certificate someone bought is a decision only
    // they can make, so it is offered second and not preselected.
    it('should offer Let\'s Encrypt second and not preselect it', async function it() {
      config.set('platform.gateway.ssl.provider', 'file');

      await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        answers: [false, false],
      });

      expect(enquirer.options[1].message).to.contain("Switch to Let's Encrypt");
      expect(enquirer.options[1].initial).to.be.false();
    });

    it('should preselect the switch for a self-signed certificate', async function it() {
      config.set('platform.gateway.ssl.provider', 'self-signed');

      await run.call(this, {
        checkGatewayCertificate: () => invalid(CERTIFICATE_REASONS.SELF_SIGNED),
        answers: [false, false],
      });

      expect(enquirer.options[1].initial).to.be.true();
    });
  });

  describe('already on Let\'s Encrypt', () => {
    it('should offer another attempt rather than a switch', async function it() {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        answers: [false],
      });

      expect(enquirer.options[0].message).to.contain('Try to obtain a new certificate');
      expect(enquirer.options[0].header).to.contain('no provider to switch to');
      expect(enquirer.options[0].header).to.contain('It is not always port');
    });
  });

  describe('unattended runs report and never act', () => {
    // A configuration change nobody asked for, made unattended on
    // infrastructure the operator owns, is not dashmate's to make - and it
    // would replace a diagnosis with a silent failure.
    it('should construct no prompt and change nothing', async function it() {
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        interactive: false,
      });

      expect(enquirer.prompt).to.not.have.been.called();
      expect(errors[0]).to.be.an.instanceOf(CertificateUnresolvedError);
      expect(obtainLetsEncryptCertificateTask).to.not.have.been.called();
      expect(configFileRepository.write).to.not.have.been.called();
    });

    // listr2 5.0.7 has no fail() on the task wrapper, so throwing is the only
    // way to show the operator which step went wrong. A green line above an
    // error message is worse than no line at all.
    it('should render the step as failed', async function it() {
      const { state } = await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        interactive: false,
      });

      expect(state).to.equal('FAILED');
    });
  });

  describe('the gateway is told about a new certificate', () => {
    // Envoy reads the certificate files once at startup. Under the documented
    // upgrade procedure the node is stopped and the new certificate loads at
    // the next start, but update against a running node is supported too, and
    // there this is what makes the change reach the wire.
    it('should reload a running gateway after a successful obtain', async function it() {
      let checked = 0;
      await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(dockerCompose.execCommand)
        .to.have.been.calledOnceWith(config, 'gateway', 'kill -SIGHUP 1');
    });

    it('should carry on when the gateway is not running', async function it() {
      dockerCompose.execCommand.rejects(new ServiceIsNotRunningError('base', 'gateway'));

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(errors).to.be.empty();
    });

    it('should not swallow a reload that failed for another reason', async function it() {
      dockerCompose.execCommand.rejects(new Error('docker daemon is unreachable'));

      let checked = 0;
      const { errors } = await run.call(this, {
        checkGatewayCertificate: () => {
          checked += 1;
          return checked === 1 ? invalid() : verdict();
        },
        answers: [true],
      });

      expect(errors[0].message).to.contain('docker daemon is unreachable');
    });
  });

  describe('the bypass suppresses enforcement, never the check', () => {
    it('should record the real status and let the run continue', async function it() {
      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => invalid(),
        skipCertificateCheck: true,
      });

      expect(errors).to.be.empty();
      expect(context.certificateSkipped).to.be.true();
      expect(context.certificate.status).to.equal(CERTIFICATE_STATUS.INVALID);
      expect(enquirer.prompt).to.not.have.been.called();
    });
  });

  describe('warnings are reported and never blocked on', () => {
    it('should carry every warning through without failing', async function it() {
      const { context, errors } = await run.call(this, {
        checkGatewayCertificate: () => verdict({
          status: CERTIFICATE_STATUS.WARN,
          warnings: [
            { code: CERTIFICATE_REASONS.PROVIDER_MISMATCH, message: 'issuer disagrees' },
            { code: CERTIFICATE_REASONS.EXPIRING_SOON, message: 'expires tomorrow' },
          ],
        }),
      });

      expect(errors).to.be.empty();

      // This node is on ZeroSSL, so the structural warning about the free-tier
      // limit rides along in front. What matters here is that neither verdict
      // warning is dropped on the way through, whatever precedes them.
      expect(context.certificateWarnings.slice(-2))
        .to.deep.equal(['issuer disagrees', 'expires tomorrow']);
    });
  });

  // ZeroSSL's obtain writes the certificate id to config before anything is
  // issued, and every caller must persist at that callback. Running it from
  // here would spend one of a free-tier operator's three lifetime certificates
  // and leave it unreferenced - on exactly the population this check exists for.
  it('should never construct the ZeroSSL obtain task', async function it() {
    const source = fs.readFileSync(
      path.join(process.cwd(), 'src/listr/tasks/update/gatewayCertificateTaskFactory.js'),
      'utf8',
    );

    expect(source).to.not.contain('obtainZeroSSLCertificateTask');
  });
});

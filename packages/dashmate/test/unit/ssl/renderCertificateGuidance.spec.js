import getBaseConfigFactory from '../../../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../../../src/config/HomeDir.js';
import renderCertificateGuidance from '../../../src/ssl/renderCertificateGuidance.js';
import { CERTIFICATE_REASONS, CERTIFICATE_STATUS } from '../../../src/ssl/checkGatewayCertificateFactory.js';

describe('renderCertificateGuidance', () => {
  let config;

  /**
   * @param {Object} [overrides]
   * @return {Object}
   */
  const verdict = (overrides = {}) => ({
    status: CERTIFICATE_STATUS.INVALID,
    reasons: [{
      code: CERTIFICATE_REASONS.EXPIRED,
      message: 'The installed certificate expired on 2026-05-01 - 111 days ago',
    }],
    warnings: [],
    skipped: [],
    provider: 'zerossl',
    installed: null,
    expiresInDays: -111,
    ...overrides,
  });

  /**
   * @param {Object} [options]
   * @return {string}
   */
  const render = (options = {}) => renderCertificateGuidance({
    config,
    verdict: verdict(),
    isNodeRunning: false,
    pull: { ok: true, failed: 0, total: 7 },
    ...options,
  });

  beforeEach(() => {
    config = getBaseConfigFactory(HomeDir.createTemp())();
    config.set('network', 'mainnet');
    config.set('externalIp', '149.28.241.190');
    config.set('platform.gateway.ssl.provider', 'zerossl');
  });

  // ConfigBaseCommand falls back to the default config when --config is absent,
  // so an operator running several nodes who pastes a bare command would obtain
  // a certificate for, restart, or bypass the check on a different node.
  it('should put the selected config on every command it prints', function it() {
    this.sinon.stub(config, 'getName').returns('testnet_2');

    const output = render();

    // Commands an operator is meant to run appear either in an indented block
    // or in backticks. Everything else naming dashmate is prose.
    const commands = [
      ...output.split('\n')
        .filter((line) => /^ {6,}dashmate /.test(line))
        .map((line) => line.trim()),
      ...(output.match(/`dashmate [^`]+`/g) ?? [])
        .map((match) => match.replace(/`/g, ''))
        // A bare `dashmate start` in prose names the command rather than
        // telling the operator to run it here.
        .filter((command) => command.split(' ').length > 2),
    ];

    // The point is that every command carries the config, not how many there
    // are - the text has deliberately got shorter.
    expect(commands).to.have.length.greaterThan(2);
    commands.forEach((command) => {
      expect(command, command).to.contain('--config testnet_2');
    });
  });

  // The printed command has to do what the interactive repair does. For a
  // certificate the checks rejected on its own contents - wrong address, not
  // valid yet - the archived copy is the rejected copy, and obtaining without
  // --force hands it straight back. An operator following this would run it
  // and find nothing changed.
  it('should force replacement for a certificate that cannot be reinstated', () => {
    [CERTIFICATE_REASONS.NOT_YET_VALID, CERTIFICATE_REASONS.IP_MISMATCH].forEach((code) => {
      const output = render({
        verdict: verdict({ reasons: [{ code, message: `rejected: ${code}` }] }),
      });

      expect(output, code).to.contain('dashmate ssl obtain --config base --provider letsencrypt --force');
    });
  });

  // Every other fault is in the copy installed for the gateway rather than in
  // the certificate, so reinstalling fixes it without spending an issuance.
  it('should not force replacement for a fault reinstalling can fix', () => {
    const output = render();

    expect(output).to.contain('dashmate ssl obtain --config base --provider letsencrypt');
    expect(output).to.not.contain('--force');
  });

  it('should shell-quote a config name that needs it', function it() {
    this.sinon.stub(config, 'getName').returns('my node');

    expect(render()).to.contain("--config 'my node'");
  });

  // The guidance is written straight to stderr rather than handed to oclif's
  // error printer, which hard-wraps at 74 columns on a non-TTY stream and would
  // break the longest remediation line mid-token into something unpastable.
  it('should never break a command across lines', () => {
    // oclif's error printer hard-wraps at the terminal width less six, which is
    // 74 on a non-TTY stderr, and it breaks mid-token. Re-wrapping the output
    // that way is what shows the hazard is real: a command an operator is meant
    // to paste does not survive it. That is why the guidance is written straight
    // to stderr and never handed to that printer.
    const WRAP_AT = 74;

    /**
     * @param {string} text
     * @return {string[]}
     */
    const commandsIn = (text) => text.split('\n')
      .filter((line) => /^ {6,}dashmate /.test(line))
      .map((line) => line.trim());

    /**
     * @param {string} text
     * @return {string}
     */
    const hardWrap = (text) => text.split('\n')
      .flatMap((line) => line.match(new RegExp(`.{1,${WRAP_AT}}`, 'g')) ?? [''])
      .join('\n');

    // Every rendered command survives as written, on every variant.
    [
      render(),
      render({ verdict: verdict({ provider: 'letsencrypt' }) }),
      render({
        verdict: verdict({
          reasons: [{
            code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
            message: 'A switch was interrupted',
          }],
        }),
      }),
    ].forEach((output) => {
      const commands = commandsIn(output);

      expect(commands).to.have.length.greaterThan(0);
      commands.forEach((command) => expect(output, command).to.contain(command));
    });

    // And at least one of them is long enough that the printer would have
    // broken it, so this is pinning a hazard that exists rather than one that
    // cannot arise.
    const switchIncomplete = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
          message: 'A switch was interrupted',
        }],
      }),
    });
    const longest = commandsIn(switchIncomplete)
      .reduce((a, b) => (b.length > a.length ? b : a));

    expect(longest.length).to.be.greaterThan(WRAP_AT - 6);
    expect(hardWrap(switchIncomplete)).to.not.contain(longest);
  });

  // The check reads files. It never opens a connection, so it cannot report
  // what clients experienced - only what a client verifying this certificate
  // would do with it. Matching on a family of phrasings rather than one exact
  // string, because the previous guard named a sentence the code never used.
  it('should not state a wire outcome it never measured', () => {
    const outputs = [
      render(),
      render({ isNodeRunning: true }),
      render({ verdict: verdict({ provider: 'letsencrypt' }) }),
      render({ pull: null }),
    ];

    outputs.forEach((output) => {
      expect(output).to.not.match(/clients (are|were|could not|cannot|unable)/i);
      expect(output).to.not.match(/(is|was|has been) unreachable/i);
      expect(output).to.not.match(/your node is (down|dark|offline)/i);

      // Nor what a client would make of the certificate itself. The chain to a
      // public root is never validated either, and some blocking findings - an
      // unfinished provider switch - sit on a certificate a client would
      // accept perfectly well.
      expect(output).to.not.match(/client (will|would|does not|will not)? ?(accept|reject)/i);
      expect(output).to.not.match(/clients? rejects?/i);
    });
  });

  // The one exception, and it is a real one rather than an oversight. Self
  // signature is proven structurally - the leaf verifies under its own public
  // key - and a certificate signed by nothing else is in no public trust store
  // by definition. That is a property of the file, established by the check,
  // not an inference about the wire.
  it('should still say a self-signed certificate is not publicly trusted', () => {
    const output = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.SELF_SIGNED,
          message: 'The installed certificate is self-signed. Self-signed TLS is not'
            + ' publicly trusted and standards-compliant clients will reject it',
        }],
      }),
    });

    expect(output).to.contain('not publicly trusted');
  });

  // Eight mainnet nodes are in this state. Telling them to switch to the
  // provider they are already on is the message they would get.
  it('should not offer a switch to a node already on that provider', () => {
    config.set('platform.gateway.ssl.provider', 'letsencrypt');

    const output = render();

    expect(output).to.contain('already uses Let\'s Encrypt');
    expect(output).to.contain("To fix it, get a new certificate");
    expect(output).to.not.contain('THE FIX - switch to');

    // The remediation itself is still the right next step and stays.
    expect(output).to.contain('dashmate ssl obtain --config base --provider letsencrypt');
  });

  // Saying the same thing three times in one message is how an operator learns
  // to skim past it.
  // Driven from the configured provider, which is what the renderer actually
  // reads. Overriding the verdict's own provider field leaves the config saying
  // something else, so the Let's Encrypt blocks never render and the test
  // measures a message no operator will ever see.
  ['zerossl', 'letsencrypt', 'file', 'self-signed'].forEach((provider) => {
    it(`should state the port 80 argument once for a ${provider} node`, () => {
      config.set('platform.gateway.ssl.provider', provider);

      const output = render();

      const occurrences = (needle) => output.split(needle).length - 1;

      expect(occurrences('reachable from the internet permanently')).to.equal(1);
    });
  });

  // A silent drop of an external probe is no information at all: 52 nodes that
  // dropped the same probe hold Let's Encrypt certificates issued within four
  // days, which is only possible over port 80. Asserting the port is blocked
  // from that is the kind of overclaim this whole design is meant to avoid.
  it('should not assert that the cluster blocks port 80', () => {
    const outputs = [render(), render({ verdict: verdict({ provider: 'letsencrypt' }) })];

    outputs.forEach((output) => {
      expect(output).to.not.match(/all three now (block|filter)/i);
      expect(output).to.not.match(/now block port 80/i);
    });

    // What the evidence does support: one operator, one day, three nodes.
    expect(render()).to.contain('reachable from the internet permanently');
  });

  // Reassurance the operator did not ask for is padding, but the failed-attempt
  // note is not reassurance - it says what may have changed.
  it('should say nothing about the update having broken anything', () => {
    expect(render()).to.not.match(/nothing broke/i);
  });

  // The promise of a future release that refuses to start reads, to a
  // masternode operator, as a threat to their collateral position.
  it('should not promise a future release that blocks start', () => {
    const output = render();

    expect(output).to.not.match(/future version|will not allow|4\.3/i);
  });

  // The documented upgrade procedure stops the node before update runs, so most
  // operators who see this have a stopped node. Reading a certificate complaint
  // and assuming it changed nothing leaves a masternode down.
  it('should lead with node state when the node is stopped', () => {
    const output = render({ isNodeRunning: false });

    expect(output).to.contain('Your node is stopped');
    expect(output).to.contain('dashmate start --config base');
  });

  // Obtaining installs the pair and signals the gateway, so a node that is
  // already up needs nothing further. Telling the operator to restart it would
  // cost them an outage for no change.
  it('should ask for nothing further when the node is running', () => {
    const output = render({ isNodeRunning: true });

    expect(output).to.not.contain('Your node is stopped');
    expect(output).to.not.contain('dashmate restart --config base');
    expect(output).to.not.contain('dashmate start --config base');
  });

  // An obtain that failed can have failed anywhere, including between writing
  // the certificate and writing the key - which replaces a working pair with a
  // mismatched one. Two of the claims here are unconditional and the code
  // cannot support either of them once that has happened.
  describe('after a remediation attempt that failed', () => {
    it('should not claim nothing changed', () => {
      const output = render({ obtainAttemptFailed: true });

      expect(output).to.contain('may have changed');
    });

    it('should not promise the node will start', () => {
      const output = render({ obtainAttemptFailed: true, isNodeRunning: false });

      expect(output).to.contain('may have changed');
    });

    it('should say nothing about a change when no attempt was made', () => {
      expect(render()).to.not.match(/may have changed/);
      expect(render({ isNodeRunning: false })).to.contain('does not prevent it starting');
    });
  });

  // Docker being unreachable, or the caller not being permitted to ask it,
  // says nothing about whether the node is up. Reporting it as stopped tells
  // the operator something false about their own node and offers them a
  // command for a state it may not be in.
  describe('when the node state could not be determined', () => {
    it('should not say the node is stopped', () => {
      const output = render({ isNodeRunning: null });

      expect(output).to.not.contain('Your node is stopped');
      expect(output).to.not.contain('dashmate start --config base');
    });

    it('should not claim a running node needs nothing further either', () => {
      const output = render({ isNodeRunning: null });

      expect(output).to.not.contain('dashmate start --config base');
    });

    it('should still give the operator the fix', () => {
      expect(render({ isNodeRunning: null })).to.contain('dashmate ssl obtain --config base');
    });
  });

  it('should say when images failed to pull', () => {
    expect(render({ pull: { ok: true, failed: 2, total: 7 } }))
      .to.contain('2 of 7 failed');
    expect(render({ pull: { ok: false, failed: 0, total: 0 } }))
      .to.contain('Images could not be pulled');
    expect(render()).to.contain('Images pulled.');
  });

  // The read-only preflight starts no pull at all, so it must not say anything
  // about one. Reporting a pull failure that never happened sends an operator
  // to look at a registry that is fine.
  it('should say nothing about images when no pull was attempted', () => {
    const output = render({ pull: null });

    expect(output).to.not.contain('Images could not be pulled');
    expect(output).to.not.contain('pulled images');
    expect(output).to.not.contain('Images pulled');
    expect(output).to.contain("This node's TLS certificate is not valid.");
  });

  // The operator's own situation, with no claim about what other providers
  // offer and no statistics about other people's nodes.
  it('should explain the ZeroSSL wall in terms of this node', () => {
    const output = render();

    expect(output).to.contain('three certificates in\n  total');
    expect(output).to.not.match(/four out of five|as of August/);
    expect(output).to.not.match(/does not issue certificates for IP addresses/);
  });

  // Half the expired Let's Encrypt nodes measured had port 80 demonstrably
  // open and stopped renewing anyway, so it is the prime suspect, not the
  // diagnosis.
  it('should name port 80 as a suspect rather than the cause', () => {
    config.set('platform.gateway.ssl.provider', 'letsencrypt');

    const output = render({ verdict: verdict({ provider: 'letsencrypt' }) });

    expect(output).to.contain('Inbound port 80 is the most common cause');

    // No claim about what any other authority does or does not issue, and no
    // narration about what dashmate has or has not looked at.
    expect(output).to.not.match(/only free one|has not read|retries renewal every hour/);
    expect(output).to.contain('dashmate logs --config base dashmate_helper');
  });

  it('should state that port 80 is permanent, never periodic', () => {
    const output = render();

    expect(output).to.contain('reachable from the internet permanently');
    expect(output).to.contain('Nothing will warn you if it lapses');
    expect(output).to.not.match(/every few days when the certificate renews/i);
  });

  // The interrupted switch needs no certificate work at all, so it gets the one
  // command that finishes the job instead of the whole port-80 argument.
  it('should name the exact repair for an interrupted switch', () => {
    const output = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
          message: 'A switch was interrupted before it finished',
        }],
      }),
    });

    expect(output).to.contain(
      'dashmate config set --config base platform.gateway.ssl.provider letsencrypt',
    );
    expect(output).to.not.contain('To fix it');
  });

  // The obtain command refuses to start without an address, so prescribing it
  // for this verdict hands the operator a command that cannot work and leaves
  // the node failing the gate with no way forward.
  it('should prescribe the address, not an obtain that cannot run', () => {
    const output = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.NO_EXTERNAL_IP,
          message: "This node's public address is not set",
        }],
      }),
    });

    expect(output).to.contain('dashmate config set --config base externalIp');
    expect(output).to.not.contain('To fix it, switch to');
  });

  // The gateway is handed the pair as-is, so a fault in the files themselves
  // stops it loading them. Telling an operator to start such a node sends them
  // to a command that fails while they are already dealing with a certificate.
  it('should not promise a stopped node will start when the files are unusable', () => {
    const output = render({
      isNodeRunning: false,
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.KEY_MISMATCH,
          message: 'The certificate and private key do not match',
        }],
      }),
    });

    expect(output).to.not.contain('does not prevent it starting');
    expect(output).to.contain('cannot start until the certificate');
  });

  it('should still offer to start a stopped node the files cannot stop', () => {
    const output = render({
      isNodeRunning: false,
      verdict: verdict({
        reasons: [{ code: CERTIFICATE_REASONS.EXPIRED, message: 'expired' }],
      }),
    });

    expect(output).to.contain('does not prevent it starting');
  });

  // Saving the provider leaves a running gateway on the certificate it already
  // had; the interactive repair signals it for exactly this reason, so the
  // printed one has to say so too or the node never picks the pair up.
  it('should load the certificate after finishing an interrupted switch', () => {
    const output = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
          message: 'A switch was interrupted before it finished',
        }],
      }),
    });

    expect(output).to.contain('dashmate restart --config base --platform');
  });

  // lego installed this certificate and only the saved provider still
  // disagrees, so the certificate itself is sound. Opening with the flat claim
  // that it is not valid sends an operator hunting a problem that is not there.
  it('should not call the certificate invalid when only the switch is unfinished', () => {
    const output = render({
      verdict: verdict({
        reasons: [{
          code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
          message: 'A switch was interrupted before it finished',
        }],
      }),
    });

    expect(output).to.not.contain("This node's TLS certificate is not valid");
    expect(output).to.contain("This node's certificate setup is unfinished");
  });

  // The installed pair being the one lego produced says nothing about whether
  // it is still valid. When something else is wrong with it too, saving the
  // setting is not the repair, and offering it as one sends the operator away
  // believing a dark node is fixed.
  it('should not offer the setting as the repair when the certificate is also broken', () => {
    const output = render({
      verdict: verdict({
        reasons: [
          { code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE, message: 'A switch was interrupted' },
          { code: CERTIFICATE_REASONS.EXPIRED, message: 'expired 158 days ago' },
        ],
      }),
    });

    expect(output).to.not.contain('Nothing needs to be obtained');
    expect(output).to.contain('To fix it');
  });

  it('should name the bypass and say it is not a playbook line', () => {
    const output = render();

    expect(output).to.contain('dashmate update --config base --skip-certificate-check');
    expect(output).to.contain('--skip-certificate-check');
  });

  describe('when a renewal failure was recorded', () => {
    it('should name the cause instead of the most likely one', () => {
      // The guess was honest while nothing recorded what happened. It is not
      // honest once something did - and half the nodes measured in this state
      // had port 80 demonstrably open.
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const output = render({
        verdict: verdict({ provider: 'letsencrypt' }),
        renewal: { code: 'PORT_80_WRONG_RESPONDER' },
      });

      expect(output).to.contain("something answered on port 80, but not this node's certificate check");
      expect(output).to.not.contain('most common cause');
      // No log stream to interpret, and a container recreation during an
      // update may already have discarded it anyway.
      expect(output).to.not.contain('dashmate logs');
    });

    it('should state the ZeroSSL limit as what happened once ZeroSSL has said so', () => {
      const output = render({
        verdict: verdict({ provider: 'zerossl' }),
        renewal: { code: 'QUOTA_EXHAUSTED' },
      });

      expect(output).to.contain('ZeroSSL will not issue another one');
    });

    it('should withhold the obtain command when the recorded cause forbids it', () => {
      // The doctor withholds the same command for the same reason. Printing it
      // here made the two surfaces disagree about the one thing the shared
      // vocabulary exists to keep consistent.
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const output = render({
        verdict: verdict({ provider: 'letsencrypt' }),
        renewal: { code: 'RATE_LIMITED' },
      });

      expect(output).to.contain('temporarily refused this address');
      expect(output).to.not.contain('ssl obtain');
    });

    it('should not invite another certificate while one is spent and unsaved', () => {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const output = render({
        verdict: verdict({ provider: 'letsencrypt' }),
        renewal: { code: 'CERTIFICATE_ISSUED_NOT_SAVED' },
      });

      expect(output).to.contain('already spent');
      expect(output).to.not.contain('ssl obtain');
    });

    it('should keep the existing text when nothing was recorded', () => {
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const output = render({ verdict: verdict({ provider: 'letsencrypt' }) });

      expect(output).to.contain('Inbound port 80 is the most common cause');
      expect(output).to.contain('dashmate logs');
    });

    it('should never render the excerpt the helper stored', () => {
      // Nothing on this path masks the operator's identity the way a collected
      // report does, so only the cause crosses over.
      config.set('platform.gateway.ssl.provider', 'letsencrypt');

      const output = render({
        verdict: verdict({ provider: 'letsencrypt' }),
        renewal: { code: 'PORT_80_UNREACHABLE', detail: 'SHOULD-NOT-APPEAR' },
      });

      expect(output).to.not.contain('SHOULD-NOT-APPEAR');
    });
  });
});

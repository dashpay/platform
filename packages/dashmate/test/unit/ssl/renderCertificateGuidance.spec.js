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

    expect(commands).to.have.length.greaterThan(4);
    commands.forEach((command) => {
      expect(command, command).to.contain('--config testnet_2');
    });
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
    expect(render()).to.contain('issued certificates on the same day');
  });

  it('should reassure that the update itself broke nothing', () => {
    expect(render()).to.contain('Nothing broke just now.');
  });

  // The promise of a future release that refuses to start reads, to a
  // masternode operator, as a threat to their collateral position.
  it('should not promise a future release that blocks start', () => {
    const output = render();

    expect(output).to.contain('This release does not block `dashmate start`');
    expect(output).to.not.match(/future version|will not allow/i);
  });

  // The documented upgrade procedure stops the node before update runs, so most
  // operators who see this have a stopped node. Reading a certificate complaint
  // and assuming it changed nothing leaves a masternode down.
  it('should lead with node state when the node is stopped', () => {
    const output = render({ isNodeRunning: false });

    expect(output).to.contain('Your node is currently stopped');
    expect(output).to.contain('dashmate start --config base');
  });

  it('should offer restart instead when the node is running', () => {
    const output = render({ isNodeRunning: true });

    expect(output).to.not.contain('Your node is currently stopped');
    expect(output).to.contain('dashmate restart --config base');
  });

  it('should say when images failed to pull', () => {
    expect(render({ pull: { ok: true, failed: 2, total: 7 } }))
      .to.contain('2 of 7 failed');
    expect(render({ pull: { ok: false, failed: 0, total: 0 } }))
      .to.contain('could not pull images');
    expect(render()).to.contain('This run pulled images, then stopped');
  });

  // The read-only preflight starts no pull at all, so it must not say anything
  // about one. Reporting a pull failure that never happened sends an operator
  // to look at a registry that is fine.
  it('should say nothing about images when no pull was attempted', () => {
    const output = render({ pull: null });

    expect(output).to.not.contain('could not pull images');
    expect(output).to.not.contain('pulled images');
    expect(output).to.not.contain('This run pulled');
    expect(output).to.contain("This node's installed TLS certificate did not pass");
  });

  it('should explain the ZeroSSL wall without blaming the operator', () => {
    const output = render();

    expect(output).to.contain('You did not\n  configure anything wrong');
    expect(output).to.contain('as of August 2026');
  });

  // Half the expired Let's Encrypt nodes measured had port 80 demonstrably
  // open and stopped renewing anyway, so it is the prime suspect, not the
  // diagnosis.
  it('should name port 80 as a suspect rather than the cause', () => {
    config.set('platform.gateway.ssl.provider', 'letsencrypt');

    const output = render({ verdict: verdict({ provider: 'letsencrypt' }) });

    expect(output).to.contain('The most likely cause is inbound port 80');
    expect(output).to.contain('It is not always port 80');
    expect(output).to.contain('dashmate logs --config base dashmate_helper');
  });

  it('should state that port 80 is permanent, never periodic', () => {
    const output = render();

    expect(output).to.contain('PORT 80 MUST STAY OPEN PERMANENTLY');
    expect(output).to.contain('goes dark within six days');
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
    expect(output).to.not.contain('THE FIX');
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
    expect(output).to.contain('THE FIX');
  });

  it('should name the bypass and say it is not a playbook line', () => {
    const output = render();

    expect(output).to.contain('dashmate update --config base --skip-certificate-check');
    expect(output).to.contain('not a line to add to a playbook');
  });
});

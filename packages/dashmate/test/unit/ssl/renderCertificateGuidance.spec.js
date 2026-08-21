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
    const output = render();

    expect(output).to.contain(
      'dashmate ssl obtain --config base --provider letsencrypt',
    );
    output.split('\n').forEach((line) => {
      expect(line, line).to.not.match(/--conf$|--provide$|dashm$/);
    });
  });

  // The check reads files on disk. It cannot know what is on the wire, whether
  // any client failed to connect, or what the helper has been doing.
  it('should claim nothing it did not observe', () => {
    const output = render();

    expect(output).to.contain('If this is the certificate the gateway is serving');
    expect(output).to.contain('dashmate did not open a connection');
    expect(output).to.not.contain('still being paid');
    expect(output).to.not.contain('clients could not connect');
    expect(output).to.not.contain('there is currently no other way');
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
    expect(render()).to.contain('pulled images, then stopped');
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

  it('should name the bypass and say it is not a playbook line', () => {
    const output = render();

    expect(output).to.contain('dashmate update --config base --skip-certificate-check');
    expect(output).to.contain('not a line to add to a playbook');
  });
});

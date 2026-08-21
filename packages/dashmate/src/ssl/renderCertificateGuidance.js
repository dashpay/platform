import { SSL_PROVIDERS } from '../constants.js';
import renderConfigFlag from '../util/renderConfigFlag.js';
import { CERTIFICATE_REASONS } from './checkGatewayCertificateFactory.js';

/**
 * How the run went for the images, said only as far as it was observed.
 *
 * A registry outage plus a bad certificate must not report that patches were
 * fetched. `updateNode` resolves a failed pull as an `error` row rather than
 * rejecting, so a run can succeed as a whole and still have delivered nothing.
 *
 * A null pull means none was attempted - the read-only preflight - and the
 * opening then says nothing about images at all, rather than reporting a
 * failure that never happened.
 *
 * @param {{ok: boolean, failed: number, total: number}|null} pull
 * @return {string}
 */
function renderOpening(pull) {
  if (pull === null || pull === undefined) {
    return "  This node's installed TLS certificate did not pass dashmate's checks.";
  }

  if (!pull.ok) {
    return `  This run could not pull images, and stopped: this node's installed TLS
  certificate did not pass dashmate's checks.`;
  }

  if (pull.failed > 0) {
    return `  This run pulled images - ${pull.failed} of ${pull.total} failed, see the table
  above - then stopped: this node's installed TLS certificate did not pass
  dashmate's checks.`;
  }

  return `  This run pulled images, then stopped: this node's installed TLS
  certificate did not pass dashmate's checks.`;
}

/**
 * @param {Object} verdict
 * @return {string}
 */
function renderObservation(verdict) {
  const [first] = verdict.reasons;

  return first ? first.message : 'the installed certificate did not pass the checks';
}

/**
 * Why a free ZeroSSL account stops working, and why it is not the operator's
 * mistake.
 *
 * @return {string}
 */
function renderZeroSslExplanation() {
  return `  Your certificate provider is ZeroSSL. dashmate's ZeroSSL integration drives
  ZeroSSL's REST API, and a free ZeroSSL account allows three certificates
  through the dashboard/API and no REST API access - so dashmate's renewals
  stop working after about 270 days. (ZeroSSL's ACME service is not a
  substitute here: it does not issue IP-address certificates.) You did not
  configure anything wrong - as of August 2026, four out of five ZeroSSL
  evonodes on mainnet were in this state.
`;
}

/**
 * Port 80 is a permanent standing requirement, and the consequence of losing it
 * is the thing operators have to hear. Phrasing it as "every few days when the
 * certificate renews" reads as a maintenance window that can be scheduled
 * around, which is how the requirement gets lost.
 *
 * @return {string}
 */
function renderPortEightyPermanence() {
  return `  PORT 80 MUST STAY OPEN PERMANENTLY - this is not a maintenance window.
  Let's Encrypt IP-address certificates last about six days, and dashmate
  renews them continuously for as long as this node runs. Every renewal needs
  inbound port 80 again.

  If you open port 80 only to get this certificate and close it afterwards, or
  if the rule does not survive a reboot, this node goes dark within six days
  and nothing will tell you. That is the most common way an evonode dies: three
  mainnet nodes issued certificates on the same day all went dark together six
  days later - one operator, one change, a whole fleet at once.

  Make the rule permanent and make sure it persists across reboots.
`;
}

/**
 * The interrupted-switch case needs no certificate work at all - the pair is
 * already installed - so it gets the one command that finishes the job rather
 * than the whole port-80 argument.
 *
 * @param {Config} config
 * @param {string} cfg
 * @return {string}
 */
function renderSwitchIncompleteGuidance(config, cfg) {
  return `  A Let's Encrypt certificate is already installed for the gateway, but the
  configuration still names ${config.get('platform.gateway.ssl.provider')}. A previous switch was interrupted
  after the files were written and before the setting was saved, so dashmate's
  helper is renewing the wrong provider.

  Nothing needs to be obtained. Finish the switch:

      dashmate config set ${cfg} platform.gateway.ssl.provider letsencrypt
`;
}

/**
 * Already on Let's Encrypt and already broken. No provider switch is offered
 * because there is nothing to switch to - Let's Encrypt is the only authority
 * that issues IP-address certificates over ACME.
 *
 * Port 80 is named as the prime suspect rather than the cause: half the nodes
 * measured in this state have port 80 demonstrably open and stopped renewing
 * regardless.
 *
 * @param {string} cfg
 * @return {string}
 */
function renderLetsEncryptDiagnosis(cfg) {
  return `  This node is already configured for Let's Encrypt, so there is no provider to
  switch to - it is the only authority that issues IP-address certificates over
  ACME. dashmate's helper is configured to retry renewal hourly, so renewal has
  most likely been failing without anyone being told. dashmate has not inspected
  the helper's history to confirm that.

  The most likely cause is inbound port 80. Let's Encrypt re-checks it on every
  renewal - roughly every four days, permanently - and a firewall rule that was
  opened once and later closed, or that did not survive a reboot, produces
  exactly this pattern.

  It is not always port 80: half the nodes in this state have port 80 open and
  stopped renewing regardless. Check the renewal logs as well:

      dashmate doctor ${cfg}
      dashmate logs ${cfg} dashmate_helper
`;
}

/**
 * @param {string} cfg
 * @param {boolean} isNodeRunning
 * @return {string}
 */
function renderFix(cfg, isNodeRunning, isAlreadyLetsEncrypt) {
  // A node already on Let's Encrypt has nothing to switch to - it is the only
  // authority that issues IP-address certificates over ACME - so the heading
  // that offers a switch would contradict the diagnosis printed above it. The
  // commands are the same either way.
  //
  // No restart follows the obtain. That command installs the pair and signals
  // the gateway, and the signal reaches Envoy's hot-restarter, which re-execs
  // Envoy against the same configuration without touching the container, so a
  // restart would cost an outage and change nothing. Starting a node that is
  // already stopped is a different thing and stays.
  const heading = isAlreadyLetsEncrypt
    ? `  THE FIX - obtain a new certificate from Let's Encrypt.`
    : `  THE FIX - switch to Let's Encrypt, which issues IP-address certificates free.`;

  return `${heading}

  Let's Encrypt proves this node owns its IP by connecting to it on inbound
  port 80. Check that first; it limits how often you may fail, so a blind
  attempt is expensive:

      dashmate doctor ${cfg}

  Then:

      dashmate ssl obtain ${cfg} --provider letsencrypt

${isNodeRunning
    ? `  That installs the certificate and signals the gateway, so a running node
  needs nothing further - no restart.
`
    : `  That installs the certificate and signals the gateway. This node is
  stopped, so bring it back up:

      dashmate start ${cfg}
`}`;
}

/**
 * Build the guidance printed after a run whose certificate was not resolved.
 *
 * Written to stderr directly and never handed to oclif's error printer: that
 * printer hard-wraps at 74 columns on a non-TTY stream, which breaks the
 * longest remediation line mid-token into something an operator cannot paste.
 *
 * Every claim is limited to what was observed. The check reads files on disk,
 * so it cannot say what is on the wire, whether clients failed to connect, or
 * what the helper has been doing.
 *
 * @param {Object} options
 * @param {Config} options.config
 * @param {Object} options.verdict
 * @param {boolean} options.isNodeRunning
 * @param {boolean} [options.obtainAttemptFailed] - an obtain was run and threw
 * @param {{ok: boolean, failed: number, total: number}|null} options.pull
 * @return {string}
 */
export default function renderCertificateGuidance({
  config,
  verdict,
  isNodeRunning,
  pull,
  obtainAttemptFailed = false,
}) {
  const cfg = renderConfigFlag(config.getName());
  const provider = config.get('platform.gateway.ssl.provider');
  // Only when the interrupted switch is the whole problem. The installed pair
  // being the one lego produced says nothing about whether it is still valid,
  // so this state can carry an expired or misaddressed certificate alongside
  // it - and there, saving the setting is not the repair.
  const isSwitchIncomplete = verdict.reasons.length === 1
    && verdict.reasons[0].code === CERTIFICATE_REASONS.SWITCH_INCOMPLETE;

  const blocks = [
    `${renderOpening(pull)}

  Node:        ${config.get('network')} (config "${config.getName()}", ${config.get('externalIp') ?? 'no external IP set'})
  Certificate: ${renderObservation(verdict)}

  These checks read the files installed for the gateway. dashmate did not open
  a connection, so it cannot say what this node is actually serving, and it did
  not validate the certificate against public trust stores either;
  \`dashmate doctor ${cfg}\` does the first of those.

  ${obtainAttemptFailed
    ? `An attempt to obtain a certificate ran just now and did not complete.
  It can have failed at any point, including after writing one half of the
  pair, so the files on disk may not be what they were before this run. The
  status above was read back from disk after the attempt, so it describes
  what is there now.`
    : `Nothing broke just now. This is the first release of dashmate that checks
  the certificate, so this is the first time you are being told.`}
`,
  ];

  // The node is normally down when this is read: the documented upgrade
  // procedure stops it before update runs. An operator who reads a certificate
  // complaint, assumes it changed nothing and walks away has left a stopped
  // masternode behind.
  if (!isNodeRunning) {
    // The reassurance holds for a certificate that merely failed the checks:
    // nothing about them gates startup. It does not hold once an obtain has
    // run and failed, because what is on disk may have changed underneath the
    // gateway, and promising a clean start there is a claim this cannot make.
    blocks.push(obtainAttemptFailed
      ? `  Your node is currently stopped. Bring it back up with \`dashmate start ${cfg}\`,
  then check it came up: the attempt above may have changed what is installed.
`
      : `  Your node is currently stopped. Run \`dashmate start ${cfg}\` to bring
  it back up - the certificate problem does not prevent it from starting.
`);
  }

  if (isSwitchIncomplete) {
    blocks.push(renderSwitchIncompleteGuidance(config, cfg));
  } else {
    if (provider === SSL_PROVIDERS.ZEROSSL) {
      blocks.push(renderZeroSslExplanation());
    }

    if (provider === SSL_PROVIDERS.LETSENCRYPT) {
      blocks.push(renderLetsEncryptDiagnosis(cfg));
    }

    blocks.push(renderFix(cfg, isNodeRunning, provider === SSL_PROVIDERS.LETSENCRYPT));
    blocks.push(renderPortEightyPermanence());

    blocks.push(`  IF YOU CANNOT OPEN PORT 80. dashmate currently has no supported alternative
  for an IP-address certificate, so there is no route from here to one issued
  by a public authority. Updates themselves are unaffected: images are
  always pulled, whatever this check finds, so this node is not being held back
  from protocol upgrades or security patches. To suppress this check for one
  run:

      dashmate update ${cfg} --skip-certificate-check

  This silences the check; it does not repair the certificate. It is an escape
  for a single run, not a line to add to a playbook.
`);
  }

  blocks.push(`  This release does not block \`dashmate start\` or \`dashmate restart\`. The
  certificate check applies only to \`dashmate update\`.
`);

  return `\n${blocks.join('\n')}\n`;
}

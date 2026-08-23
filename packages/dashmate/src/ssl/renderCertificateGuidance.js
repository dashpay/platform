import { SSL_PROVIDERS } from '../constants.js';
import renderConfigFlag from '../util/renderConfigFlag.js';
import { CERTIFICATE_REASONS, requiresReplacement } from './checkGatewayCertificateFactory.js';

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
    return "  This node's TLS certificate did not pass dashmate's checks.";
  }

  if (!pull.ok) {
    return `  Images could not be pulled, and this node's TLS certificate did not pass
  dashmate's checks.`;
  }

  const pulled = pull.failed > 0
    ? `Images pulled, ${pull.failed} of ${pull.total} failed - see the table above.`
    : 'Images pulled.';

  return `  ${pulled} This node's TLS certificate did not pass
  dashmate's checks.`;
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
  return `  This node uses ZeroSSL. A free ZeroSSL account allows three certificates in
  total, so renewals stop working after about 270 days.
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
  return `  Keep inbound port 80 reachable from the internet permanently, for
  certificate reissue. Nothing will warn you if it lapses.
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
  return `  A Let's Encrypt certificate is installed, but the configuration still says
  ${config.get('platform.gateway.ssl.provider')}. Nothing needs to be obtained - finish the switch:

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
  return `  This node already uses Let's Encrypt, so there is no provider to switch to.
  Inbound port 80 is the most common cause. Check the renewal logs:

      dashmate logs ${cfg} dashmate_helper
`;
}

/**
 * @param {string} cfg
 * @param {boolean} isAlreadyLetsEncrypt
 * @param {Object} verdict - decides whether the certificate can be reinstated
 * @return {string}
 */
function renderFix(cfg, isAlreadyLetsEncrypt, verdict) {
  // A node already on Let's Encrypt has nothing to switch to, so the heading
  // that offers a switch would contradict the diagnosis above it.
  const heading = isAlreadyLetsEncrypt
    ? "  THE FIX - obtain a new certificate from Let's Encrypt."
    : "  THE FIX - switch to Let's Encrypt. Certificates are free.";

  return `${heading}

  This needs inbound port 80 reachable from the internet. Check it first:

      dashmate doctor ${cfg}

  Then:

      dashmate ssl obtain ${cfg} --provider letsencrypt${requiresReplacement(verdict) ? ' --force' : ''}
`;
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
 * @param {boolean|null} options.isNodeRunning - null when it could not be
 *   determined, in which case nothing is said about the node's state
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
${obtainAttemptFailed
    ? `
  An attempt to obtain a certificate just failed part way through, so what is
  installed may have changed. The status above was read after that attempt.
`
    : ''}`,
  ];

  // Only when the state is known. Docker being unreachable establishes nothing,
  // and telling an operator their running node is stopped is worse than
  // saying nothing.
  if (isNodeRunning === false) {
    blocks.push(`  Your node is stopped. The certificate does not prevent it starting:

      dashmate start ${cfg}
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

    blocks.push(renderFix(cfg, provider === SSL_PROVIDERS.LETSENCRYPT, verdict));
    blocks.push(renderPortEightyPermanence());

    blocks.push(`  Cannot open port 80? There is no other way to get an IP-address
  certificate. Images are pulled either way, so this node is not held back. To
  skip this check for one run:

      dashmate update ${cfg} --skip-certificate-check
`);
  }

  return `\n${blocks.join('\n')}\n`;
}

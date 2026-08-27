import { SSL_PROVIDERS } from '../constants.js';
import renderConfigFlag from '../util/renderConfigFlag.js';
import { CERTIFICATE_REASONS, requiresReplacement } from './checkGatewayCertificateFactory.js';
import deriveRenewalGuidance, { ISSUANCE_STATUS, SAFE_ACTION } from './renewalGuidance.js';
import renderObtainCommand from './renderObtainCommand.js';

/**
 * Faults in the files themselves. The gateway is handed the pair as-is, so any
 * of these stops it loading - `saveCertificateTask` refuses to leave such a
 * pair behind for exactly this reason. Everything else (expiry, a wrong
 * address, a self-signed authority) is a certificate clients reject, on a
 * gateway that starts and serves it perfectly well.
 */
const GATEWAY_CANNOT_LOAD = [
  CERTIFICATE_REASONS.BUNDLE_MISSING,
  CERTIFICATE_REASONS.BUNDLE_UNREADABLE,
  CERTIFICATE_REASONS.BUNDLE_ORDER,
  CERTIFICATE_REASONS.KEY_MISSING,
  CERTIFICATE_REASONS.KEY_UNUSABLE,
  CERTIFICATE_REASONS.KEY_MISMATCH,
];

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
 * The interrupted switch is the one state where the certificate itself is
 * sound - lego installed it and only the saved provider still disagrees - so
 * calling it invalid would send an operator hunting a certificate problem that
 * is not there.
 *
 * @param {{ok: boolean, failed: number, total: number}|null} pull
 * @param {boolean} isSwitchIncomplete
 * @return {string}
 */
function renderOpening(pull, isSwitchIncomplete) {
  const subject = isSwitchIncomplete
    ? "this node's certificate setup is unfinished"
    : "this node's TLS certificate is not valid";
  const sentence = subject.charAt(0).toUpperCase() + subject.slice(1);

  if (pull === null || pull === undefined) {
    return `  ${sentence}.`;
  }

  if (!pull.ok) {
    return `  Images could not be pulled, and ${subject}.`;
  }

  const pulled = pull.failed > 0
    ? `Images pulled, ${pull.failed} of ${pull.total} failed - see the table above.`
    : 'Images pulled.';

  return `  ${pulled} ${sentence}.`;
}

/**
 * @param {Object} verdict
 * @return {string}
 */
function renderObservation(verdict) {
  const [first] = verdict.reasons;

  return first ? first.message : 'the installed certificate is not usable';
}

/**
 * Why a free ZeroSSL account stops working, and why it is not the operator's
 * mistake.
 *
 * @return {string}
 */
function renderZeroSslExplanation(renewal) {
  // Once ZeroSSL has actually said so, this stops being background about how
  // the free tier works and becomes what happened to this node.
  // Only where ZeroSSL actually said so. Printed against an unrelated failure
  // - an unreachable API, an interrupted renewal - it reads as the diagnosis
  // and sends an operator to switch provider over something transient.
  if (renewal?.code === 'PROVIDER_PLAN_REQUIRED') {
    return `  This node uses ZeroSSL, and ${renewal.cause}.
`;
  }

  if (renewal?.code === 'QUOTA_EXHAUSTED') {
    return `  This node uses ZeroSSL, and its free account has used all three of its
  certificates - so ZeroSSL will not issue another one.
`;
  }

  if (renewal?.cause) {
    return `  This node uses ZeroSSL. Renewal is failing: ${renewal.cause}.
`;
  }

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
  ${config.get('platform.gateway.ssl.provider')}. Nothing needs to be obtained - save the setting,
  then load the certificate that is already there:

      dashmate config set ${cfg} platform.gateway.ssl.provider letsencrypt
      dashmate restart ${cfg} --platform
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
function renderLetsEncryptDiagnosis(cfg, renewal) {
  // Only a guess while nothing recorded what happened. With a record there is
  // no reason to name a likely cause, and no reason to send an operator to a
  // log stream that a container recreation may already have discarded.
  if (renewal?.cause) {
    return `  This node already uses Let's Encrypt, so there is no provider to switch to.
  Renewal is failing: ${renewal.cause}.
`;
  }

  return `  This node already uses Let's Encrypt, so there is no provider to switch to.
  Inbound port 80 is the most common cause. Check the renewal logs:

      dashmate logs ${cfg} dashmate_helper
`;
}

/**
 * Without an address there is nothing to put in a certificate, and the obtain
 * command refuses to start - so prescribing it here would hand an operator a
 * command that cannot work. The address is the repair; the certificate follows
 * once one exists.
 *
 * @param {string} cfg
 * @return {string}
 */
function renderNoExternalIpGuidance(cfg, mayObtain, configName, guidance) {
  // The address is required either way. The request that follows it is not
  // exempt from the decision every other request goes through - an issuance
  // already outstanding is still outstanding once the address is set.
  const request = mayObtain
    ? `      ${renderObtainCommand({ configName, guidance })}\n`
    : '';

  return `  To fix it, tell dashmate this node's public address${mayObtain ? `, then get a
  certificate for it` : ''}:

      dashmate config set ${cfg} externalIp <your-public-ip>
${request}`;
}

/**
 * @param {string} cfg
 * @param {Object} renewal
 * @return {string}
 */
function renderWithheldObtain(cfg, renewal) {
  if (renewal.issuanceStatus === ISSUANCE_STATUS.SPENT) {
    return `  A certificate was issued and could not be saved, so it is already spent
  against this node's limit and asking again spends another. Check free space
  and permissions where dashmate saves certificates first:

      dashmate doctor ${cfg}
`;
  }

  // Not the same claim. dashmate does not know whether a certificate exists,
  // and saying one could not be saved would assert something it cannot.
  if (renewal.issuanceStatus === ISSUANCE_STATUS.UNCERTAIN) {
    return `  An earlier attempt may already have been issued a certificate without dashmate
  seeing it, so asking again could spend a second one. Check whether one
  arrived first:

      dashmate doctor ${cfg}
`;
  }

  return `  Do not obtain a certificate right now - it would not succeed, and each
  attempt counts against this node's limits. Check again once the cause above
  has cleared:

      dashmate doctor ${cfg}
`;
}

/**
 * @param {string} cfg
 * @param {boolean} isAlreadyLetsEncrypt
 * @param {Object} verdict - decides whether the certificate can be reinstated
 * @return {string}
 */
function renderFix(cfg, isAlreadyLetsEncrypt, verdict, configName, guidance) {
  // A node already on Let's Encrypt has nothing to switch to, so the heading
  // that offers a switch would contradict the diagnosis above it.
  const heading = isAlreadyLetsEncrypt
    ? "  To fix it, get a new certificate from Let's Encrypt."
    : "  To fix it, switch to Let's Encrypt. Certificates are free.";

  return `${heading}

  This needs inbound port 80 reachable from the internet. Check it first:

      dashmate doctor ${cfg}

  Then:

      ${renderObtainCommand({
    configName, guidance, force: requiresReplacement(verdict),
  })}
`;
}

/**
 * The node still works and renewal comes back around by itself, so a command
 * here spends one of the few failed attempts this node is allowed on a repair
 * that has not been made yet.
 *
 * @param {string} cfg
 * @return {string}
 */
function renderFixLocallyThenWait(cfg) {
  return `  Fix the cause above. dashmate retries by itself, so there is no command to
  run - check it worked afterwards:

      dashmate doctor ${cfg}
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
 * @param {Object|null} [options.renewal] - the recorded renewal failure, when
 *   one applies to the certificate this node is using
 * @param {Object} options.renewal - derived once by deriveRenewalGuidance
 * @return {string}
 */
export default function renderCertificateGuidance({
  config,
  verdict,
  isNodeRunning,
  pull,
  obtainAttemptFailed = false,
  renewal = null,
}) {
  const cfg = renderConfigFlag(config.getName());
  // A caller with nothing recorded still gets a decision, so no branch below
  // has to work out for itself what an absent record means.
  const guidance = renewal ?? deriveRenewalGuidance({
    hasNoExternalIp: verdict.reasons
      .some(({ code }) => code === CERTIFICATE_REASONS.NO_EXTERNAL_IP),
    isCertificateUsable: false,
  });
  const provider = config.get('platform.gateway.ssl.provider');
  // Only when the interrupted switch is the whole problem. The installed pair
  // being the one lego produced says nothing about whether it is still valid,
  // so this state can carry an expired or misaddressed certificate alongside
  // it - and there, saving the setting is not the repair.
  const isSwitchIncomplete = verdict.reasons.length === 1
    && verdict.reasons[0].code === CERTIFICATE_REASONS.SWITCH_INCOMPLETE;

  const blocks = [
    `${renderOpening(pull, isSwitchIncomplete)}

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
    // Only when the files themselves are sound. Telling an operator to start a
    // node whose gateway cannot load the pair sends them to a command that
    // fails, at the moment they are already dealing with a broken certificate.
    const cannotLoad = verdict.reasons.some(({ code }) => GATEWAY_CANNOT_LOAD.includes(code));

    blocks.push(cannotLoad
      ? `  Your node is stopped, and the gateway cannot start until the certificate
  files are repaired.
`
      : `  Your node is stopped. The certificate does not prevent it starting:

      dashmate start ${cfg}
`);
  }

  if (isSwitchIncomplete) {
    blocks.push(renderSwitchIncompleteGuidance(config, cfg));
  } else {
    if (provider === SSL_PROVIDERS.ZEROSSL) {
      blocks.push(renderZeroSslExplanation(guidance));
    }

    if (provider === SSL_PROVIDERS.LETSENCRYPT) {
      blocks.push(renderLetsEncryptDiagnosis(cfg, guidance));
    }

    // The address is a prerequisite for every other repair, so it is said
    // first and regardless - the obtain command refuses to start without one.
    if (guidance.prerequisites.includes('EXTERNAL_IP')) {
      blocks.push(renderNoExternalIpGuidance(
        cfg,
        guidance.safeAction !== SAFE_ACTION.DO_NOT_OBTAIN,
        config.getName(),
        guidance,
      ));
    }

    if (guidance.safeAction === SAFE_ACTION.DO_NOT_OBTAIN) {
      blocks.push(renderWithheldObtain(cfg, guidance));
    } else if (guidance.safeAction === SAFE_ACTION.WAIT_AFTER_LOCAL_FIX) {
      blocks.push(renderFixLocallyThenWait(cfg));
    } else if (!guidance.prerequisites.includes('EXTERNAL_IP')) {
      blocks.push(renderFix(
        cfg,
        provider === SSL_PROVIDERS.LETSENCRYPT,
        verdict,
        config.getName(),
        guidance,
      ));
    }

    blocks.push(renderPortEightyPermanence());

    blocks.push(`  Cannot open port 80? There is no other way to get an IP-address
  certificate. Images are pulled either way, so this node is not held back. To
  skip this check for one run:

      dashmate update ${cfg} --skip-certificate-check
`);
  }

  return `\n${blocks.join('\n')}\n`;
}

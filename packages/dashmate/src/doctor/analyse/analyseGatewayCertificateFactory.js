import chalk from 'chalk';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';
import renderConfigFlag from '../../util/renderConfigFlag.js';
import {
  CERTIFICATE_REASONS,
  GATED_NETWORKS,
  requiresReplacement,
} from '../../ssl/checkGatewayCertificateFactory.js';
import { isRenewalRecordCurrent, RENEWAL_OUTCOMES, RENEWAL_RECORD_STATES } from '../../ssl/renewalRecord.js';
import { describeRenewalFailure, REMEDY_CLASS } from '../../ssl/renewalFailure.js';
import { RETRY_INTERVAL_MS } from '../../helper/scheduleRenewalJob.js';
import { SSL_PROVIDERS } from '../../constants.js';

/**
 * The manual obtain command writes certificate files but does not signal the gateway, so an
 * operator following the advice can succeed and see no change on the wire. Every message about
 * a certificate the gateway has not picked up has to say this.
 *
 * The node is named because a report is read against one config among several, and a command
 * pasted without one acts on whichever happens to be the default.
 *
 * Only for a remedy that changes the files by hand. Anything routed through
 * `dashmate ssl obtain` needs no restart: that command installs the pair and
 * signals the gateway, and the signal reaches Envoy's hot-restarter, which
 * re-execs Envoy against the same configuration without touching the
 * container. Measured against a live gateway, not inferred.
 *
 * @param {string} cfg
 * @return {string}
 */
/**
 * Plain wording for the connection failures a probe can report.
 *
 * The codes come from Node and from OpenSSL, and an operator reading a doctor
 * report has no way to look them up. Anything not listed falls through to the
 * code itself rather than being softened into something vaguer - an unfamiliar
 * code is still searchable, whereas "something went wrong" is not.
 */
const CONNECTION_FAILURES = {
  ETIMEDOUT: 'nothing answered in time',
  ECONNREFUSED: 'the connection was refused',
  EHOSTUNREACH: 'the address could not be reached',
  ENETUNREACH: 'the network could not be reached',
  ECONNRESET: 'the connection was closed before it finished',
  NO_PEER_CERTIFICATE: 'it answered but offered no certificate',
  CONNECT_FAILED: 'the connection could not be made',
};

/**
 * Plain wording for why a served certificate is not trusted.
 *
 * Same rule as above: translate what is known, pass through what is not.
 */
const TRUST_FAILURES = {
  CERT_HAS_EXPIRED: 'it has expired',
  DEPTH_ZERO_SELF_SIGNED_CERT: 'it is self-signed, so no certificate authority vouches for it',
  SELF_SIGNED_CERT_IN_CHAIN: 'it is self-signed, so no certificate authority vouches for it',
  // Only one certificate arriving is established by this code. Why its issuer
  // could not be found is not, so both readings are named.
  UNABLE_TO_VERIFY_LEAF_SIGNATURE: 'only one certificate was sent and its issuer could not be'
    + ' found - either the ones that vouch for it are missing, or this machine does not trust'
    + ' the authority that issued it',
  UNABLE_TO_GET_ISSUER_CERT: 'the certificate that issued it could not be found - either it was'
    + ' not sent with the others, or this machine does not trust it',
  // Returned for a complete, correct bundle signed by a root the machine does
  // not trust just as readily as for one that is genuinely missing
  // certificates, so it must not be read as either on its own.
  UNABLE_TO_GET_ISSUER_CERT_LOCALLY: 'no trusted path could be built to it - either certificates'
    + ' are missing from the bundle, or this machine does not trust the authority that issued it',
  CERT_NOT_YET_VALID: 'its start date is in the future',
};

/**
 * @param {Object} table
 * @param {string} code
 * @return {string}
 */
const describe = (table, code) => table[code] ?? code;

/**
 * The verification failures that are about the chain of trust itself - a
 * missing issuer, or an authority nothing vouches for. A certificate can also
 * fail verification while its chain is perfectly sound, because the dates do
 * not hold; saying the authority is untrusted there is simply false, and sends
 * an operator to replace a certificate when the clock is what is wrong.
 */
const TRUST_PATH_FAILURES = [
  'DEPTH_ZERO_SELF_SIGNED_CERT',
  'SELF_SIGNED_CERT_IN_CHAIN',
  'UNABLE_TO_VERIFY_LEAF_SIGNATURE',
  'UNABLE_TO_GET_ISSUER_CERT',
  'UNABLE_TO_GET_ISSUER_CERT_LOCALLY',
];

const restartHint = (cfg) => chalk`Then restart Platform so the gateway picks it up: {bold.cyanBright dashmate restart ${cfg} --platform}`;

/**
 * An operator reading a certificate problem is deciding whether their node is
 * falling behind. It is not: `update` pulls images whatever the certificate
 * does, and only refuses to report success. Leaving this out lets a client
 * reachability problem be read as a software delivery one.
 */
const UPDATE_CONSEQUENCE = 'The certificate saved for the gateway is not usable.'
  + ' Updates still work.';

/**
 * Renewal only means something where dashmate is the one renewing.
 *
 * The shipped default is SSL turned off with a provider already named, so
 * reading the provider alone would speak on every node that has never obtained
 * a certificate, and on every node whose operator deliberately stopped.
 *
 * @param {Config} config
 * @return {boolean}
 */
const isRenewalManaged = (config) => config.get('platform.gateway.ssl.enabled') === true
  && [SSL_PROVIDERS.ZEROSSL, SSL_PROVIDERS.LETSENCRYPT]
    .includes(config.get('platform.gateway.ssl.provider'));

/**
 * @param {string|null} value
 * @return {string|null}
 */
const asDay = (value) => (value ? new Date(value).toISOString().slice(0, 10) : null);

/**
 * When the certificate in use stops working, which is the only number that
 * tells an operator how much time they have.
 *
 * @param {Object|null} installed
 * @return {string}
 */
function renderDeadline(installed) {
  const day = asDay(installed?.validTo);

  return day ? ` This node stops accepting clients on ${day}.` : '';
}

/**
 * What is known about how long this has been going on.
 *
 * Never "failing since" the last success: the record knows when renewal last
 * worked and how many attempts have failed since, not when the failures began,
 * and on a ninety-day certificate those are months apart. The count itself is
 * not shown either - it counts scheduler wake-ups, which mix hourly re-checks
 * with attempts days apart, so a number here would be read as attempts.
 *
 * @param {Object} record
 * @return {string}
 */
function renderHistory(record) {
  const lastSuccess = asDay(record.lastSuccessAt);

  return lastSuccess
    ? `Last renewed ${lastSuccess}. Every attempt since has failed.`
    : 'dashmate does not know when this node last renewed successfully.';
}

/**
 * Whether renewal will come back around on its own, and when.
 *
 * Derived rather than stored, so it cannot promise a retry that was recorded
 * before anything decided there would be one. A time already past is reported
 * as such - it is also the plainest evidence available that the part of
 * dashmate which renews certificates is not running.
 *
 * @param {Object} record
 * @param {number} now
 * @return {string}
 */
function renderNextAttempt(record, now) {
  const nextAt = new Date(record.attemptedAt).getTime() + RETRY_INTERVAL_MS;

  if (nextAt <= now) {
    return 'dashmate was due to try again and has not, so the part of dashmate that renews'
      + ' certificates may not be running.';
  }

  return `dashmate tries again by itself at ${new Date(nextAt).toISOString().slice(11, 16)} UTC.`;
}

/**
 * The ending an operator is given, chosen by what the cause allows.
 *
 * A cause that cannot be repaired by asking again must never end in a command
 * that asks again: the certificate authority limits how often this node may
 * fail, and an issuance that was spent but never landed is spent whether or not
 * it arrived. This is why the remedy is carried with the cause rather than
 * written beside it.
 *
 * @param {Object} options
 * @return {string}
 */
function renderRemedy({
  remedy, cfg, force, isIssuanceSpent, isCertificateUsable,
}) {
  const obtain = chalk`{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${force}}`;

  if (isIssuanceSpent) {
    return chalk`Do not obtain another certificate yet - one was already issued and is spent
against this node's limit of five a week, so asking again spends another.

Check free space and permissions where dashmate saves certificates, then:
${obtain}`;
  }

  if (remedy === REMEDY_CLASS.DO_NOT_RETRY) {
    return chalk`Do not obtain a certificate right now - it would be refused the same way and
count against this node's limits. Wait, then check again:
{bold.cyanBright dashmate doctor ${cfg}}`;
  }

  if (remedy === REMEDY_CLASS.SWITCH_PROVIDER) {
    return chalk`Switch to Let's Encrypt. Certificates are free and it does not cap the number
of certificates this way. It needs inbound port 80 open to the internet,
permanently - and if you cannot open it, there is no other way to get a
certificate for an IP address.
${obtain}`;
  }

  if (remedy === REMEDY_CLASS.FIX_LOCALLY && isCertificateUsable) {
    // The node still works, and renewal comes back around on its own once the
    // cause is gone. Ending here with a command spends one of the few failed
    // attempts this node is allowed, on a fix that has not been made yet.
    return null;
  }

  if (remedy === REMEDY_CLASS.WAIT && isCertificateUsable) {
    return null;
  }

  return chalk`Get a working certificate:
${obtain}`;
}

/**
 * Where to look for whatever is holding port 80.
 *
 * `ss` lists what is listening on this machine, which is the whole answer only
 * when dashmate's own check could not bind. When something answered the
 * certificate authority instead, it is as likely to be a router forwarding the
 * port elsewhere, or a hosting provider's page - and an operator who sees an
 * empty table and stops has nowhere else to look.
 *
 * @param {string} code
 * @return {string}
 */
function renderPortEightyHint(code) {
  if (code === 'PORT_80_IN_USE') {
    return chalk`Find what is using port 80 on this machine and move it off that port:
{bold.cyanBright sudo ss -lntp 'sport = :80'}`;
  }

  if (code === 'PORT_80_WRONG_RESPONDER') {
    return chalk`Another web server, a proxy, or your router is answering instead of this node.
Check this machine first:
{bold.cyanBright sudo ss -lntp 'sport = :80'}
Nothing listed? Then it is answered before it reaches this machine - check your
router's port forwarding and your hosting provider.`;
  }

  return `Open inbound port 80 - on the machine's firewall, at your hosting provider, and on
your router if this node is behind one. It has to stay open: the certificate is
renewed every few days.`;
}

export default function analyseGatewayCertificateFactory() {
  /**
   * Analyse the certificate installed for the gateway and the one it serves.
   *
   * @typedef analyseGatewayCertificate
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analyseGatewayCertificate(samples) {
    const config = samples.getDashmateConfig();

    if (!config?.get('platform.enable')) {
      return [];
    }

    // `update` enforces on these networks and only these. A local or devnet
    // node serves a self-signed certificate by design, so diagnosing one here
    // would report a healthy node as broken and prescribe a certificate no
    // authority can issue for an address it cannot reach.
    if (!GATED_NETWORKS.includes(config.get('network'))) {
      return [];
    }

    const cfg = renderConfigFlag(config.getName());

    const problems = [];

    // The gateway is stopped whenever the documented upgrade procedure is
    // followed, and a stopped gateway answers no TLS connection - so the probe
    // below records nothing and every problem with the files on disk would go
    // unreported, on exactly the node an operator has just been told to run
    // doctor on.
    const installed = samples.getServiceInfo('gateway', 'installedCertificate');

    // Reinstalling cannot fix an address the certificate does not carry, or a
    // start date still ahead, and the reuse check is weaker than the one that
    // rejected it - so an unforced command hands the same certificate back.
    // Every remedy in this analyser reads this one decision: printing a forced
    // command beside an unforced one tells an operator two different things
    // about the same certificate.
    const installedForce = requiresReplacement(installed) ? ' --force' : '';

    // Certificate validity is judged against the moment the samples were taken.
    const sampledAt = samples.date?.getTime() ?? Date.now();

    // Only a record that still describes the certificate in use. A provider
    // switch leaves the previous provider's account behind, and a certificate
    // obtained by hand after a failure overtakes that failure entirely - the
    // helper cannot notice either, so the reader has to.
    const renewalSample = samples.getServiceInfo('gateway', 'certificateRenewal');
    const renewal = isRenewalManaged(config)
      && renewalSample?.state === RENEWAL_RECORD_STATES.PRESENT
      && isRenewalRecordCurrent(renewalSample, {
        provider: config.get('platform.gateway.ssl.provider'),
        certificateValidFrom: installed?.validFrom ? new Date(installed.validFrom) : null,
      })
      ? renewalSample
      : null;

    const failedRenewal = renewal?.outcome === RENEWAL_OUTCOMES.FAILED ? renewal : null;

    /**
     * What the record says went wrong, and what to do about it.
     *
     * @param {boolean} isCertificateUsable
     * @return {string}
     */
    const renderRenewalCause = (isCertificateUsable) => {
      const { remedy } = describeRenewalFailure(failedRenewal.code);
      const isIssuanceSpent = Boolean(failedRenewal.issuanceSpentAt);
      const blocks = [];

      if (remedy === REMEDY_CLASS.FIX_LOCALLY) {
        blocks.push(renderPortEightyHint(failedRenewal.code));
      }

      const ending = renderRemedy({
        remedy,
        cfg,
        force: installedForce,
        isIssuanceSpent,
        isCertificateUsable,
      });

      if (ending) {
        blocks.push(ending);
      }

      if (isCertificateUsable) {
        blocks.push(chalk`${renderNextAttempt(failedRenewal, sampledAt)} Then check it worked:
{bold.cyanBright dashmate doctor ${cfg}}`);
      }

      if (failedRenewal.detail && failedRenewal.code === 'UNKNOWN') {
        blocks.push(`It reported: ${failedRenewal.detail}`);
      }

      return blocks.join('\n\n');
    };

    if (installed) {
      installed.reasons.forEach(({ code, message }) => {
        // Nothing can be issued for an address dashmate does not have, and the
        // obtain command refuses to start without one, so the address has to
        // be set before a certificate is worth asking for.
        const remedy = code === CERTIFICATE_REASONS.NO_EXTERNAL_IP
          ? chalk`${UPDATE_CONSEQUENCE}

Set this node's public address, then obtain a certificate:
{bold.cyanBright dashmate config set ${cfg} externalIp <your-public-ip>}
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`
          : chalk`${UPDATE_CONSEQUENCE}

Obtain a new certificate. No restart needed:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${installedForce}}`;

        problems.push(new Problem(
          message,
          failedRenewal
            ? `${remedy}\n\nRenewal is failing: ${describeRenewalFailure(failedRenewal.code).sentence}.`
            : remedy,
          SEVERITY.HIGH,
        ));
      });

      // Fires on a node every other check calls healthy. Nothing is wrong with
      // the certificate in use; it is simply the last one this node will get
      // unless the cause is repaired, and on a Let's Encrypt certificate that
      // is a couple of days away.
      const isCertificateUsable = installed.status !== 'INVALID';

      if (failedRenewal && isCertificateUsable) {
        problems.push(new Problem(
          `This node's certificate is not being renewed:`
          + ` ${describeRenewalFailure(failedRenewal.code).sentence}.`
          + `${renderDeadline(installed)} ${renderHistory(failedRenewal)}`,
          renderRenewalCause(true),
          SEVERITY.HIGH,
        ));
      }

      if (renewal?.gatewayReloadFailedAt) {
        problems.push(new Problem(
          `This node's certificate was renewed on ${asDay(renewal.lastSuccessAt)}, but the gateway`
          + ' is still using the old one',
          chalk`Load it without an outage:
{bold.cyanBright dashmate ssl obtain ${cfg}}`,
          SEVERITY.HIGH,
        ));
      }

      installed.warnings.forEach(({ message }) => {
        problems.push(new Problem(
          message,
          chalk`Nothing is broken yet. If it needs attention, obtain a new certificate:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${installedForce}}`,
          SEVERITY.LOW,
        ));
      });
    }

    const served = samples.getServiceInfo('gateway', 'servedCertificate');

    if (!served) {
      return problems;
    }

    if (served.state === 'unreachable') {
      problems.push(new Problem(
        "The gateway's own listener did not answer a secure connection:"
        + ` ${describe(CONNECTION_FAILURES, served.reason)}. Clients may not be able to connect`,
        chalk`Please check that the gateway is running and listening: {bold.cyanBright dashmate status ${cfg} platform}`,
        SEVERITY.MEDIUM,
      ));

      return problems;
    }

    if (served.state !== 'served') {
      return problems;
    }

    const externalIp = config.get('externalIp');

    // An identity mismatch is evaluated first and stops the comparisons below. It means the
    // connection did not reach this node's gateway at all - another config or a proxy answering
    // on the same port - and in that case the certificate it returned says nothing about this
    // node, so reporting it as a wrong or stale certificate would be misleading.
    if (served.identityVerified === false) {
      // No restart here, and no unconditional reissue. If something else is
      // answering on that port, a new certificate installs on a gateway nobody
      // is reaching and the port stays taken - the operator would take an
      // outage and still have the problem. Reissuing is the remedy only once
      // this node's gateway is known to be what answered.
      problems.push(new Problem(
        `The certificate being served on port ${served.port} is not issued for this`
        + ` node's address, ${externalIp}`,
        chalk`Something other than this node's gateway may be answering on port ${served.port} -
another dashmate config, a reverse proxy, or a second node. Find what is
listening there first.

If this node's gateway is answering and the address is simply wrong:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt --force}`,
        SEVERITY.HIGH,
      ));

      return problems;
    }

    const servedExpiresAt = new Date(served.certificate.validTo).getTime();
    const now = sampledAt;
    const isServedExpired = servedExpiresAt <= now;
    const onDiskDiffers = served.matchesOnDisk === false;

    // A restart makes the gateway load whatever is on disk, so it may only be
    // advised once the disk copy is known to be better on every count that
    // matters. Outliving what is on the wire is necessary and nowhere near
    // sufficient: the wire sample carries a fingerprint and a date, while
    // whether the pair matches its key, names this address, or is self-signed
    // comes from the checks run over the files in the same collection.
    const onDiskExpiresAt = served.onDisk
      ? new Date(served.onDisk.validTo).getTime()
      : null;
    const isOnDiskNewer = onDiskExpiresAt !== null && onDiskExpiresAt > servedExpiresAt;
    //
    // Fails closed. An absent verdict is not a passing one - a report collected
    // by an older dashmate carries none at all - and neither is one that merely
    // stopped short of failing. The verdict must also be about the pair the
    // probe measured: the two samples are taken moments apart, and a renewal
    // landing between them means the file that was judged is not the file that
    // would be loaded.
    const isOnDiskUsable = isOnDiskNewer
      && onDiskExpiresAt > now
      && installed?.status === 'CHECKS_PASSED'
      && Boolean(installed.fingerprint256)
      && installed.fingerprint256 === served.onDisk?.fingerprint256;

    if (isServedExpired && onDiskDiffers && isOnDiskUsable) {
      problems.push(new Problem(
        `This node is using a certificate that expired on ${served.certificate.validTo}. `
        + 'A newer one has already been saved and is ready to use',
        chalk`The new certificate was saved but the node never picked it up. Load it:
{bold.cyanBright dashmate restart ${cfg} --platform}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired && onDiskDiffers) {
      problems.push(new Problem(
        `This node is using a certificate that expired on ${served.certificate.validTo}. `
        + 'A different one has been saved, but dashmate could not confirm it is a working '
        + 'replacement',
        chalk`Neither the certificate in use nor the saved one is known to work, so
restarting will not help. Get a current one:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${installedForce}}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired) {
      problems.push(new Problem(
        `This node is using a certificate that expired on ${served.certificate.validTo}. `
        + 'Clients cannot connect to it',
        failedRenewal
          ? chalk`Renewal is failing: ${describeRenewalFailure(failedRenewal.code).sentence}.

${renderRenewalCause(false)}`
          : chalk`Renewal has not succeeded. Check the logs, then obtain a new certificate:
{bold.cyanBright dashmate logs ${cfg} dashmate_helper}
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${installedForce}}`,
        SEVERITY.HIGH,
      ));
    } else if (onDiskDiffers) {
      if (isOnDiskUsable) {
        // Still serving a valid certificate, but the renewed one has not been picked up, so this
        // node goes dark when the served certificate expires.
        problems.push(new Problem(
          'This node is using an older certificate than the one that has been saved. '
          + `It will stop accepting clients on ${served.certificate.validTo}`,
          chalk`The new certificate was saved but the node never picked it up. Load it:
{bold.cyanBright dashmate restart ${cfg} --platform}`,
          SEVERITY.HIGH,
        ));
      } else {
        problems.push(new Problem(
          'This node is using a different certificate from the one that has been saved, and '
          + 'dashmate could not confirm the saved one is a working replacement',
          chalk`The certificate in use works. The saved one is not known to be a safe
replacement, so do not restart to load it. Get a current one instead:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt${installedForce}}`,
          SEVERITY.HIGH,
        ));
      }
    }

    // Reported separately from expiry because the connection surfaces only its first
    // verification failure: a certificate that is both expired and untrusted reports only the
    // expiry, and the second fault would otherwise stay hidden until the first was fixed.
    if (!served.chainVerified && !isServedExpired) {
      problems.push(new Problem(
        'The certificate this node is serving is not trusted by ordinary clients:'
        + ` ${describe(TRUST_FAILURES, served.chainError)}`,
        TRUST_PATH_FAILURES.includes(served.chainError)
          ? chalk`Standard clients will reject this node.

If the bundle is missing the certificates that vouch for the server one, add them.
${restartHint(cfg)}

If the bundle is already complete, the authority that issued it is not one clients
trust, and no restart changes that. Get a publicly trusted certificate:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`
          : chalk`Standard clients will reject this node. The chain itself is not the
problem, so adding certificates to the bundle will not help. Check this node's
clock first. If the clock is right, the certificate's own dates are wrong and it
has to be replaced:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt --force}`,
        SEVERITY.HIGH,
      ));
    }

    // Nothing is said about inbound port 80 here on purpose. The sample comes
    // from a connect test, which measures whether something is listening - and
    // nothing listens on port 80 on a healthy node except for the seconds a
    // renewal takes, so it reports closed on healthy nodes by construction.
    // Alongside a certificate problem it reads as the cause of that problem, to
    // exactly the operators least able to tell a real firewall fault from this
    // phantom one, and sends them to rewrite rules that are already correct.
    // A drop carries no information; only an answer or a refusal does.

    return problems;
  }

  return analyseGatewayCertificate;
}

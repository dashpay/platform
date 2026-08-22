import chalk from 'chalk';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';
import renderConfigFlag from '../../util/renderConfigFlag.js';

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

const restartHint = (cfg) => chalk`Then restart Platform so the gateway picks it up: {bold.cyanBright dashmate restart ${cfg} --platform}`;

/**
 * An operator reading a certificate problem is deciding whether their node is
 * falling behind. It is not: `update` pulls images whatever the certificate
 * does, and only refuses to report success. Leaving this out lets a client
 * reachability problem be read as a software delivery one.
 */
const UPDATE_CONSEQUENCE = 'The gateway certificate did not pass dashmate\'s checks.'
  + ' `dashmate update` still pulls images, but exits non-zero until this is fixed.';

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

    const cfg = renderConfigFlag(config.getName());

    const problems = [];

    // The gateway is stopped whenever the documented upgrade procedure is
    // followed, and a stopped gateway answers no TLS connection - so the probe
    // below records nothing and every problem with the files on disk would go
    // unreported, on exactly the node an operator has just been told to run
    // doctor on.
    const installed = samples.getServiceInfo('gateway', 'installedCertificate');

    if (installed) {
      installed.reasons.forEach(({ message }) => {
        problems.push(new Problem(
          message,
          chalk`${UPDATE_CONSEQUENCE}

Obtain a new certificate. No restart needed:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`,
          SEVERITY.HIGH,
        ));
      });

      installed.warnings.forEach(({ message }) => {
        problems.push(new Problem(
          message,
          chalk`Nothing is broken yet. If it needs attention, obtain a new certificate:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`,
          SEVERITY.LOW,
        ));
      });
    }

    const served = samples.getServiceInfo('gateway', 'servedCertificate');

    if (!served) {
      return problems;
    }

    // Certificate validity is judged against the moment the samples were taken, not the moment
    // they are analysed. A report is often opened days after it was collected, and the node's
    // certificate may be renewed every few days, so judging at analysis time would report every
    // healthy node as expired.
    const now = samples.date?.getTime() ?? Date.now();

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
{bold.cyanBright dashmate ssl obtain ${cfg} --force}`,
        SEVERITY.HIGH,
      ));

      return problems;
    }

    const servedExpiresAt = new Date(served.certificate.validTo).getTime();
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
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired) {
      problems.push(new Problem(
        `This node is using a certificate that expired on ${served.certificate.validTo}. `
        + 'Clients cannot connect to it',
        chalk`Renewal has not succeeded. Check the logs, then obtain a new certificate:
{bold.cyanBright dashmate logs ${cfg} dashmate_helper}
{bold.cyanBright dashmate ssl obtain ${cfg}}`,
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
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`,
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
        chalk`Standard clients will reject this node. If the chain is incomplete, make sure
the bundle contains the issuing certificates as well as the server one.
${restartHint(cfg)}`,
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

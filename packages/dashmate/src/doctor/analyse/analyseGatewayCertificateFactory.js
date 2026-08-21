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
 * @param {string} cfg
 * @return {string}
 */
const restartHint = (cfg) => chalk`Then restart Platform so the gateway picks it up: {bold.cyanBright dashmate restart ${cfg} --platform}`;

/**
 * An operator reading a certificate problem is deciding whether their node is
 * falling behind. It is not: `update` pulls images whatever the certificate
 * does, and only refuses to report success. Leaving this out lets a client
 * reachability problem be read as a software delivery one.
 */
const UPDATE_CONSEQUENCE = 'The certificate installed for the gateway did not pass dashmate\'s'
  + ' checks. `dashmate update` still pulls new images, so protocol upgrades and security patches'
  + ' continue to arrive - but it exits non-zero until this is fixed.';

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

Obtain a new certificate - it signals the gateway itself, so no restart is needed:
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
        `The gateway did not answer a TLS connection (${served.reason}). Clients may not be able to connect`,
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
      problems.push(new Problem(
        `The certificate served on port ${served.port} is not valid for ${externalIp}: ${served.identityError}`,
        chalk`Either the certificate is issued for the wrong address, or something other than this
node's gateway is answering on that port. Check that no other node or proxy is using it, then
regenerate the certificate if needed: {bold.cyanBright dashmate ssl obtain ${cfg} --force}
${restartHint(cfg)}`,
        SEVERITY.HIGH,
      ));

      return problems;
    }

    const servedExpiresAt = new Date(served.certificate.validTo).getTime();
    const isServedExpired = servedExpiresAt <= now;
    const onDiskDiffers = served.matchesOnDisk === false;

    // Which of the two is newer decides both the description and whether a
    // restart is safe to advise, on either branch below. A restart makes the
    // gateway load the disk copy, so advising one without checking can replace
    // what is on the wire with something no better - or worse.
    const onDiskExpiresAt = served.onDisk
      ? new Date(served.onDisk.validTo).getTime()
      : null;
    const isOnDiskNewer = onDiskExpiresAt !== null && onDiskExpiresAt > servedExpiresAt;

    if (isServedExpired && onDiskDiffers && isOnDiskNewer) {
      problems.push(new Problem(
        `The gateway is serving a certificate that expired on ${served.certificate.validTo}, `
        + 'while a newer one is already present on disk',
        chalk`The certificate was renewed but never reached the gateway.
{bold.cyanBright dashmate restart ${cfg} --platform}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired && onDiskDiffers) {
      problems.push(new Problem(
        `The gateway is serving a certificate that expired on ${served.certificate.validTo}. `
        + 'The copy on disk is a different one, and it is no newer',
        chalk`Neither the certificate on the wire nor the one on disk is usable, so restarting
Platform would not help. Obtain a current certificate, which installs it and
signals the gateway:
{bold.cyanBright dashmate ssl obtain ${cfg} --provider letsencrypt}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired) {
      problems.push(new Problem(
        `The gateway is serving a certificate that expired on ${served.certificate.validTo}. `
        + 'Clients cannot connect to this node',
        chalk`Renewal has not succeeded. Check the renewal logs:
{bold.cyanBright dashmate logs ${cfg} dashmate_helper}
Then obtain a new certificate: {bold.cyanBright dashmate ssl obtain ${cfg}}
${restartHint(cfg)}`,
        SEVERITY.HIGH,
      ));
    } else if (onDiskDiffers) {
      if (isOnDiskNewer) {
        // Still serving a valid certificate, but the renewed one has not been picked up, so this
        // node goes dark when the served certificate expires.
        problems.push(new Problem(
          'The gateway is serving an older certificate than the one on disk. '
          + `It will stop accepting clients on ${served.certificate.validTo}`,
          chalk`The certificate was renewed but never reached the gateway.
{bold.cyanBright dashmate restart ${cfg} --platform}`,
          SEVERITY.HIGH,
        ));
      } else {
        problems.push(new Problem(
          'The gateway is serving a different certificate from the one on disk, and the one '
          + 'on disk is not the newer of the two',
          chalk`Whatever is on the wire is currently the better of the two, so do not restart
Platform to load the file - that would replace it with the older one. Obtain a
current certificate first, which also installs it and signals the gateway:
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
        `The certificate served by the gateway is not trusted by standard clients (${served.chainError})`,
        chalk`Clients verifying against public certificate authorities will reject this node.
If the certificate chain is incomplete, make sure the bundle contains the issuing
certificates as well as the server certificate.
${restartHint(cfg)}`,
        SEVERITY.HIGH,
      ));
    }

    // Nothing is said about inbound port 80 here on purpose. The sample comes
    // from a connect test, which measures whether something is listening - and
    // nothing listens on port 80 on a healthy node except for the seconds a
    // renewal takes, so it reports closed on healthy nodes by construction.
    // Reporting it alongside certificate problems put the claim in front of
    // exactly the operators least able to tell a real firewall problem from a
    // phantom one, and sent them to rewrite rules that were already correct.
    // A drop carries no information; only an answer or a refusal does.

    return problems;
  }

  return analyseGatewayCertificate;
}

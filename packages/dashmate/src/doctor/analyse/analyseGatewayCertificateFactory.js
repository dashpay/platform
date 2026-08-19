import chalk from 'chalk';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

/**
 * The manual obtain command writes certificate files but does not signal the gateway, so an
 * operator following the advice can succeed and see no change on the wire. Every message about
 * a certificate the gateway has not picked up has to say this.
 */
const RESTART_HINT = chalk`Then restart the node so the gateway picks it up: {bold.cyanBright dashmate restart}`;

export default function analyseGatewayCertificateFactory() {
  /**
   * Analyse the certificate the gateway actually serves.
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

    const served = samples.getServiceInfo('gateway', 'servedCertificate');

    if (!served) {
      return [];
    }

    const problems = [];

    // Certificate validity is judged against the moment the samples were taken, not the moment
    // they are analysed. A report is often opened days after it was collected, and the node's
    // certificate may be renewed every few days, so judging at analysis time would report every
    // healthy node as expired.
    const now = samples.date?.getTime() ?? Date.now();

    if (served.state === 'unreachable') {
      problems.push(new Problem(
        `The gateway did not answer a TLS connection (${served.reason}). Clients may not be able to connect`,
        chalk`Please check that the gateway is running and listening: {bold.cyanBright dashmate status platform}`,
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
regenerate the certificate if needed: {bold.cyanBright dashmate ssl obtain --force}
${RESTART_HINT}`,
        SEVERITY.HIGH,
      ));

      return problems;
    }

    const servedExpiresAt = new Date(served.certificate.validTo).getTime();
    const isServedExpired = servedExpiresAt <= now;
    const onDiskDiffers = served.matchesOnDisk === false;

    if (isServedExpired && onDiskDiffers) {
      problems.push(new Problem(
        `The gateway is serving a certificate that expired on ${served.certificate.validTo}, `
        + 'while a newer one is already present on disk',
        chalk`The certificate was renewed but never reached the gateway.
{bold.cyanBright dashmate restart}`,
        SEVERITY.HIGH,
      ));
    } else if (isServedExpired) {
      problems.push(new Problem(
        `The gateway is serving a certificate that expired on ${served.certificate.validTo}. `
        + 'Clients cannot connect to this node',
        chalk`Renewal has not succeeded. Check the renewal logs:
{bold.cyanBright dashmate logs dashmate_helper}
Then obtain a new certificate: {bold.cyanBright dashmate ssl obtain}
${RESTART_HINT}`,
        SEVERITY.HIGH,
      ));
    } else if (onDiskDiffers) {
      // Still serving a valid certificate, but the renewed one has not been picked up, so this
      // node goes dark when the served certificate expires.
      problems.push(new Problem(
        'The gateway is serving an older certificate than the one on disk. '
        + `It will stop accepting clients on ${served.certificate.validTo}`,
        chalk`The certificate was renewed but never reached the gateway.
{bold.cyanBright dashmate restart}`,
        SEVERITY.HIGH,
      ));
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
${RESTART_HINT}`,
        SEVERITY.HIGH,
      ));
    }

    // Both obtainable providers reach this node on port 80 to validate it. Being closed is
    // only reported alongside a certificate problem: the port is bound just for the seconds a
    // validation takes, so an external check finds it closed on healthy nodes too and on its
    // own would be noise.
    const validationHttpPort = samples.getServiceInfo('gateway', 'validationHttpPort');

    if (problems.length > 0 && validationHttpPort && validationHttpPort !== 'OPEN') {
      problems.push(new Problem(
        'Inbound port 80 is not reachable, which is how certificates are validated. '
        + 'This may be why renewal is failing',
        chalk`Please make sure port 80 on ${externalIp} accepts incoming connections from the
internet. Both certificate providers connect back to it to validate this node's
address before issuing a certificate. If you are behind NAT, forward port 80 as well.`,
        SEVERITY.MEDIUM,
      ));
    }

    return problems;
  }

  return analyseGatewayCertificate;
}

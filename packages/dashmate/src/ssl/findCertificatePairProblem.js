import selectLeafCertificate from './selectLeafCertificate.js';

/**
 * Why the gateway cannot serve TLS from a certificate bundle and its private
 * key, if it cannot
 *
 * The pair has to load and match the way the gateway loads it, with the key's
 * certificate first, and that certificate must not have expired: clients
 * refuse an expired one.
 *
 * @param {string} bundlePem
 * @param {string} privateKeyPem
 * @param {Date} [now]
 * @return {string|null} the reason, or null when it can
 */
export default function findCertificatePairProblem(bundlePem, privateKeyPem, now = new Date()) {
  const selection = selectLeafCertificate(bundlePem, privateKeyPem);

  if (selection.error) {
    return selection.detail;
  }

  if (new Date(selection.leaf.validTo) <= now) {
    return `its certificate expired on ${selection.leaf.validTo}`;
  }

  return null;
}

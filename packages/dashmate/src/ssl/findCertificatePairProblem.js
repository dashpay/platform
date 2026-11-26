import fs from 'fs';
import selectLeafCertificate from './selectLeafCertificate.js';
import { isSelfSigned, readValidityWindow } from './leafCertificateChecks.js';

/**
 * Why the gateway cannot serve TLS from a certificate bundle and its private
 * key, if it cannot
 *
 * The pair has to load and match the way the gateway loads it, with the key's
 * certificate first, and that certificate must be inside its validity window:
 * clients refuse one that has expired or is not valid yet. Unless it is
 * self-signed, the bundle must also hold
 * the certificate that issued it. ZeroSSL writes the key's certificate and
 * then its authority's into one file, and a write a full disk cuts short right
 * after the first block still parses, but clients cannot build a chain from it.
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

  const [leaf, ...others] = selection.certificates;

  const validity = readValidityWindow(leaf, now);

  if (validity.isNotYetValid) {
    return `its certificate is not valid until ${leaf.validFrom}`;
  }

  if (validity.isExpired) {
    return `its certificate expired on ${leaf.validTo}`;
  }

  const isIssuerPresent = isSelfSigned(leaf)
    || others.some((certificate) => leaf.checkIssued(certificate)
      && leaf.verify(certificate.publicKey));

  if (!isIssuerPresent) {
    return 'the bundle does not hold the certificate that issued the key\'s certificate,'
      + ' so it is incomplete or was cut short';
  }

  return null;
}

/**
 * @param {string} filePath
 * @return {boolean}
 */
function isRegularFile(filePath) {
  try {
    return fs.statSync(filePath, { throwIfNoEntry: false })?.isFile() === true;
  } catch {
    return false;
  }
}

/**
 * Why the gateway cannot serve TLS from the certificate files at these paths,
 * if it cannot
 *
 * A file this process cannot read is a reason too: what cannot be read here
 * cannot be checked. Only regular files are opened: opening a named pipe
 * that nothing writes to never returns.
 *
 * @param {string} bundleFilePath
 * @param {string} privateKeyFilePath
 * @param {Date} [now]
 * @return {string|null} the reason, or null when it can
 */
export function findCertificateFilesProblem(bundleFilePath, privateKeyFilePath, now = new Date()) {
  const notAFile = [bundleFilePath, privateKeyFilePath]
    .find((filePath) => !isRegularFile(filePath));

  if (notAFile !== undefined) {
    return `${notAFile} is missing or is not a regular file`;
  }

  let bundlePem;
  let privateKeyPem;

  try {
    bundlePem = fs.readFileSync(bundleFilePath, 'utf8');
    privateKeyPem = fs.readFileSync(privateKeyFilePath, 'utf8');
  } catch (e) {
    return e.message;
  }

  return findCertificatePairProblem(bundlePem, privateKeyPem, now);
}

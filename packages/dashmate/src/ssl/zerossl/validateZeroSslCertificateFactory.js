import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import containsPrivateKey from '../containsPrivateKey.js';
import findCertificatePairProblem from '../findCertificatePairProblem.js';
import loadCertificateRequest from './loadCertificateRequest.js';
import selectLeafCertificate, { findPrivateKeyProblem } from '../selectLeafCertificate.js';
import readRegularFile from '../../util/readRegularFile.js';

export const ERRORS = {
  API_KEY_IS_NOT_SET: 'API_KEY_IS_NOT_SET',
  EXTERNAL_IP_IS_NOT_SET: 'EXTERNAL_IP_IS_NOT_SET',
  CERTIFICATE_ID_IS_NOT_SET: 'CERTIFICATE_ID_IS_NOT_SET',
  PRIVATE_KEY_IS_NOT_PRESENT: 'PRIVATE_KEY_IS_NOT_PRESENT',
  EXTERNAL_IP_MISMATCH: 'EXTERNAL_IP_MISMATCH',
  CSR_FILE_IS_NOT_PRESENT: 'CSR_FILE_IS_NOT_PRESENT',
  CERTIFICATE_EXPIRES_SOON: 'CERTIFICATE_EXPIRES_SOON',
  CERTIFICATE_IS_NOT_VALIDATED: 'CERTIFICATE_IS_NOT_VALIDATED',
  CERTIFICATE_IS_NOT_VALID: 'CERTIFICATE_IS_NOT_VALID',
  ZERO_SSL_API_ERROR: 'ZERO_SSL_API_ERROR',
};

/**
 * Whether a validation result leaves the saved certificate to finish rather
 * than to replace
 *
 * An obtain without --force downloads a certificate ZeroSSL issued again,
 * and validates one ZeroSSL still validates, without asking for another. The
 * validation reaches these results only when the private key the certificate
 * was requested with is on this node and the address matches.
 *
 * @param {string|undefined} error - from validateZeroSslCertificate
 * @return {boolean}
 */
export function isSavedCertificateReusable(error) {
  return error === undefined || error === ERRORS.CERTIFICATE_IS_NOT_VALIDATED;
}

/**
 * ZeroSSL's error code for a certificate ID it does not know
 */
const CERTIFICATE_NOT_FOUND = 2803;

/**
 * Whether ZeroSSL answered that it has no certificate with the saved ID
 *
 * Nothing is left to finish then, and only a new certificate restores TLS.
 *
 * @param {string|undefined} error - from validateZeroSslCertificate
 * @param {Object} data - from validateZeroSslCertificate
 * @return {boolean}
 */
export function isSavedCertificateNotFound(error, data) {
  return error === ERRORS.ZERO_SSL_API_ERROR && data.error?.code === CERTIFICATE_NOT_FOUND;
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
 * Whether the private key a certificate was requested with is on disk and
 * loads
 *
 * Compose creates a missing bind-mount source as a directory, and a full
 * disk can cut the key short. A key this process cannot read counts: the
 * gateway may read what this process cannot.
 *
 * @param {string} filePath
 * @return {boolean}
 */
function isPrivateKeyUsable(filePath) {
  if (!isRegularFile(filePath)) {
    return false;
  }

  let privateKeyPem;
  try {
    privateKeyPem = fs.readFileSync(filePath, 'utf8');
  } catch {
    return true;
  }

  return findPrivateKeyProblem(privateKeyPem) === null;
}

/**
 * The certificate request on disk, and whether it can renew the certificate
 *
 * Renewal sends the request to ZeroSSL, so it has to parse, be signed by its
 * own key, and carry the public key of the private key on disk, an RSA or an
 * EC one: a full disk can cut it short, and a request for another key asks
 * for a certificate this node cannot serve. A file that also holds a private
 * key cannot be used either: ZeroSSL and doctor reports get the request, and
 * never a key. The file is read once, and what renewal sends is the request
 * checked here alone.
 *
 * A request that is not a regular file, or that this process cannot read, is
 * left as it is, and so is one whose key this process cannot read, once it
 * parses. Only a request read here can be sent.
 *
 * @param {string} csrFilePath
 * @param {string} privateKeyFilePath
 * @return {{ isUsable: boolean, pem: (string|undefined) }} pem - the request
 *  renewal sends, when one was read and can be used
 */
function readCertificateRequest(csrFilePath, privateKeyFilePath) {
  const leftAsIs = { isUsable: true, pem: undefined };
  const unusable = { isUsable: false, pem: undefined };

  let stats;
  try {
    stats = fs.statSync(csrFilePath, { throwIfNoEntry: false });
  } catch {
    return leftAsIs;
  }

  if (stats === undefined) {
    return unusable;
  }

  let text;
  try {
    const file = readRegularFile(csrFilePath);

    if (file === null) {
      return leftAsIs;
    }

    // Node cannot turn a file of about 512 MiB into a string
    text = file.content.toString('latin1');
  } catch {
    return leftAsIs;
  }

  if (containsPrivateKey(text)) {
    return unusable;
  }

  const request = loadCertificateRequest(text);

  if (request === null) {
    return unusable;
  }

  const usable = { isUsable: true, pem: request.pem };

  if (!isRegularFile(privateKeyFilePath)) {
    return usable;
  }

  let privateKeyPem;
  try {
    privateKeyPem = fs.readFileSync(privateKeyFilePath, 'utf8');
  } catch {
    return usable;
  }

  // Given the file, Node would read a certificate that follows the key
  try {
    return request.publicKey.equals(crypto.createPublicKey(crypto.createPrivateKey(privateKeyPem)))
      ? usable
      : unusable;
  } catch {
    return unusable;
  }
}

/**
 * @param {string|null|undefined} fingerprint - hexadecimal, separators and
 *  case aside
 * @return {string|null}
 */
function normalizeFingerprint(fingerprint) {
  if (typeof fingerprint !== 'string') {
    return null;
  }

  const hex = fingerprint.replace(/[^0-9a-f]/gi, '').toLowerCase();

  return hex.length > 0 ? hex : null;
}

/**
 * Whether an obtain leaves the bundle on disk as it is rather than download
 * the certificate again
 *
 * Only the bundle of the saved certificate is kept: one whose first
 * certificate has the SHA-1 fingerprint ZeroSSL reports for the saved ID.
 * Renewal saves the new ID before it installs the new bundle, and the bundle
 * of the certificate it replaces still serves until it expires. Without
 * ZeroSSL's fingerprint nothing ties the bundle to the saved ID, and it is
 * downloaded by that ID. A missing bundle is downloaded, and so is a pair of
 * regular files this process reads and the gateway could not serve: one a
 * full disk cut short, or one that belongs to another key. Anything else is
 * left for the obtain's last check to report. A file this process cannot
 * read may be one the gateway reads, and a named pipe blocks a read and a
 * write alike.
 *
 * @param {string} bundleFilePath
 * @param {string} privateKeyFilePath
 * @param {string|null|undefined} fingerprint - ZeroSSL's for the saved ID
 * @return {boolean}
 */
function isBundleKept(bundleFilePath, privateKeyFilePath, fingerprint) {
  let stats;
  try {
    stats = fs.statSync(bundleFilePath, { throwIfNoEntry: false });
  } catch {
    return true;
  }

  if (stats === undefined) {
    return false;
  }

  if (!stats.isFile() || !isRegularFile(privateKeyFilePath)) {
    return true;
  }

  let bundlePem;
  let privateKeyPem;
  try {
    bundlePem = fs.readFileSync(bundleFilePath, 'utf8');
    privateKeyPem = fs.readFileSync(privateKeyFilePath, 'utf8');
  } catch {
    return true;
  }

  if (findCertificatePairProblem(bundlePem, privateKeyPem) !== null) {
    return false;
  }

  const expected = normalizeFingerprint(fingerprint);
  const { leaf } = selectLeafCertificate(bundlePem, privateKeyPem);

  return expected !== null && normalizeFingerprint(leaf.fingerprint) === expected;
}

/**
 * @param {HomeDir} homeDir
 * @param {getCertificate} getCertificate
 * @return {validateZeroSslCertificate}
 */
export default function validateZeroSslCertificateFactory(homeDir, getCertificate) {
  /**
   * @typedef {validateZeroSslCertificate}
   * @param {Config} config
   * @param {number} expirationDays
   * @param {Object} [options]
   * @param {AbortSignal} [options.signal] - ends the request to ZeroSSL
   * @return {Promise<{ [error: String], [data: Object] }>}
   */
  async function validateZeroSslCertificate(config, expirationDays, { signal } = {}) {
    const data = {};

    data.sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    data.csrFilePath = path.join(data.sslConfigDir, 'csr.pem');
    data.privateKeyFilePath = path.join(data.sslConfigDir, 'private.key');
    data.bundleFilePath = path.join(data.sslConfigDir, 'bundle.crt');

    data.apiKey = config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey');

    if (!data.apiKey) {
      return {
        error: ERRORS.API_KEY_IS_NOT_SET,
        data,
      };
    }

    data.externalIp = config.get('externalIp');

    if (!data.externalIp) {
      return {
        error: ERRORS.EXTERNAL_IP_IS_NOT_SET,
        data,
      };
    }

    const certificateId = config.get('platform.gateway.ssl.providerConfigs.zerossl.id');

    if (!certificateId) {
      return {
        error: ERRORS.CERTIFICATE_ID_IS_NOT_SET,
        data,
      };
    }

    // Certificate is already configured

    // Check if certificate files are present. Only what they show is kept
    // here: doctor archives this data.
    const certificateRequest = readCertificateRequest(data.csrFilePath, data.privateKeyFilePath);
    data.isCsrFilePresent = certificateRequest.isUsable;
    data.isPrivateKeyFilePresent = isPrivateKeyUsable(data.privateKeyFilePath);

    // This function will throw an error if certificate with specified ID is not present
    try {
      data.certificate = await getCertificate(data.apiKey, certificateId, { signal });
    } catch (e) {
      if (e.code) {
        data.error = e;

        return {
          error: ERRORS.ZERO_SSL_API_ERROR,
          data,
        };
      }

      throw e;
    }

    data.isBundleFilePresent = data.isPrivateKeyFilePresent
      && isBundleKept(data.bundleFilePath, data.privateKeyFilePath, data.certificate.fingerprint_sha1);

    data.isExpiresSoon = data.certificate.isExpiredInDays(expirationDays);

    // If certificate exists but private key does not, then we can't setup TLS connection
    // In this case we need to regenerate a certificate or put back this private key
    if (!data.isPrivateKeyFilePresent) {
      return {
        error: ERRORS.PRIVATE_KEY_IS_NOT_PRESENT,
        data,
      };
    }

    // We need to make sure that external IP and certificate IP match
    if (data.certificate.common_name !== data.externalIp) {
      return {
        error: ERRORS.EXTERNAL_IP_MISMATCH,
        data,
      };
    }

    if (['pending_validation', 'draft'].includes(data.certificate.status)) {
      // We need to validate and download new certificate bundle
      data.isBundleFilePresent = false;

      return {
        error: ERRORS.CERTIFICATE_IS_NOT_VALIDATED,
        data,
      };
    }

    // Certificate is already created at this point, so we just need to pass validation
    // and download certificate file

    if (data.certificate.status !== 'issued' || data.isExpiresSoon) {
      // Certificate is going to expire soon, or current certificate is not valid
      // we need to obtain a new one

      // We need to download new certificate bundle
      data.isBundleFilePresent = false;

      if (certificateRequest.pem === undefined) {
        return {
          error: ERRORS.CSR_FILE_IS_NOT_PRESENT,
          data,
        };
      }

      data.csr = certificateRequest.pem;

      return {
        error: data.isExpiresSoon
          ? ERRORS.CERTIFICATE_EXPIRES_SOON
          : ERRORS.CERTIFICATE_IS_NOT_VALID,
        data,
      };
    }

    // Certificate is valid, so we might need only to download certificate bundle
    return {
      data,
    };
  }

  return validateZeroSslCertificate;
}

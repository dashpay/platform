import fs from 'fs';
import path from 'path';

import LegoCertificate from './LegoCertificate.js';
import isCertificatePairInstalled from './isCertificatePairInstalled.js';

export const ERRORS = {
  EXTERNAL_IP_IS_NOT_SET: 'EXTERNAL_IP_IS_NOT_SET',
  CERTIFICATE_NOT_FOUND: 'CERTIFICATE_NOT_FOUND',
  PRIVATE_KEY_NOT_FOUND: 'PRIVATE_KEY_NOT_FOUND',
  CERTIFICATE_EXPIRES_SOON: 'CERTIFICATE_EXPIRES_SOON',
  CERTIFICATE_IP_MISMATCH: 'CERTIFICATE_IP_MISMATCH',
  CERTIFICATE_NOT_VALID: 'CERTIFICATE_NOT_VALID',
  CERTIFICATE_NOT_INSTALLED: 'CERTIFICATE_NOT_INSTALLED',
};

/**
 * @param {HomeDir} homeDir
 * @return {validateLetsEncryptCertificate}
 */
export default function validateLetsEncryptCertificateFactory(homeDir) {
  /**
   * @typedef {validateLetsEncryptCertificate}
   * @param {Config} config
   * @param {number} expirationDays
   * @return {Promise<{ [error: String], [data: Object] }>}
   */
  async function validateLetsEncryptCertificate(
    config,
    expirationDays = LegoCertificate.EXPIRATION_LIMIT_DAYS,
  ) {
    const data = {};

    // SSL output directory (where we copy final certs for gateway)
    data.sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    data.privateKeyFilePath = path.join(data.sslConfigDir, 'private.key');
    data.bundleFilePath = path.join(data.sslConfigDir, 'bundle.crt');

    // Lego data directory (where lego stores its state)
    data.legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');

    // Reported for the caller's information only. A contact address is optional
    // under RFC 8555, Let's Encrypt stopped sending expiry notifications in
    // 2025, and nothing in dashmate asks for one - so no new node has one and
    // refusing here would fail every check on every new node.
    data.email = config.get('platform.gateway.ssl.providerConfigs.letsencrypt.email');

    data.externalIp = config.get('externalIp');

    if (!data.externalIp) {
      return {
        error: ERRORS.EXTERNAL_IP_IS_NOT_SET,
        data,
      };
    }

    // Lego output paths
    data.legoCertPath = path.join(data.legoDir, 'certificates', `${data.externalIp}.crt`);
    data.legoKeyPath = path.join(data.legoDir, 'certificates', `${data.externalIp}.key`);

    // Check if lego certificate files exist
    data.isLegoCertPresent = fs.existsSync(data.legoCertPath);
    data.isLegoKeyPresent = fs.existsSync(data.legoKeyPath);

    // Check if gateway SSL files exist
    data.isPrivateKeyFilePresent = fs.existsSync(data.privateKeyFilePath);
    data.isBundleFilePresent = fs.existsSync(data.bundleFilePath);
    data.isCertificatePairInstalled = isCertificatePairInstalled(
      data.legoCertPath,
      data.legoKeyPath,
      data.bundleFilePath,
      data.privateKeyFilePath,
    );

    if (!data.isLegoCertPresent) {
      return {
        error: ERRORS.CERTIFICATE_NOT_FOUND,
        data,
      };
    }

    if (!data.isLegoKeyPresent) {
      return {
        error: ERRORS.PRIVATE_KEY_NOT_FOUND,
        data,
      };
    }

    // Parse certificate to check expiration
    try {
      data.certificate = LegoCertificate.fromFile(data.legoCertPath);
    } catch (e) {
      return {
        error: ERRORS.CERTIFICATE_NOT_VALID,
        data,
      };
    }

    data.isExpiresSoon = data.certificate.isExpiredInDays(expirationDays);
    data.expirationDays = expirationDays;

    // Check if certificate IP matches external IP
    // First check SANs (preferred for IP certificates with --disable-cn)
    // Fall back to commonName if no IP SANs present
    const certIpAddresses = data.certificate.ipAddresses;
    const hasMatchingIp = certIpAddresses.length > 0
      ? certIpAddresses.includes(data.externalIp)
      : data.certificate.commonName === data.externalIp;

    if (!hasMatchingIp) {
      return {
        error: ERRORS.CERTIFICATE_IP_MISMATCH,
        data,
      };
    }

    // Check if certificate is still valid
    if (!data.certificate.isValid()) {
      return {
        error: ERRORS.CERTIFICATE_NOT_VALID,
        data,
      };
    }

    // Check if certificate expires soon
    if (data.isExpiresSoon) {
      return {
        error: ERRORS.CERTIFICATE_EXPIRES_SOON,
        data,
      };
    }

    // The certificate is valid, but the gateway loads its own copy rather than the issued
    // file. Until the two match the node keeps serving whatever was installed last, which
    // stays invisible to every check that only looks at the issued certificate.
    if (!data.isCertificatePairInstalled) {
      return {
        error: ERRORS.CERTIFICATE_NOT_INSTALLED,
        data,
      };
    }

    // Certificate is valid
    return {
      data,
    };
  }

  return validateLetsEncryptCertificate;
}

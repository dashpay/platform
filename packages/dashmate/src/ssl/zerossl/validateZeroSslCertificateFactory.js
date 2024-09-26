import fs from 'fs';
import path from 'path';

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
 * @param {HomeDir} homeDir
 * @param {getCertificate} getCertificate
 * @return {validateZeroSslCertificate}
 */
export default function validateZeroSslCertificateFactory(homeDir, getCertificate) {
  /**
   * @typedef {validateZeroSslCertificate}
   * @param {Config} config
   * @param {number} expirationDays
   * @return {Promise<{ [error: String], [data: Object] }>}
   */
  async function validateZeroSslCertificate(config, expirationDays) {
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

    // Check if certificate files are present
    data.isCsrFilePresent = fs.existsSync(data.csrFilePath);
    data.isPrivateKeyFilePresent = fs.existsSync(data.privateKeyFilePath);
    data.isBundleFilePresent = fs.existsSync(data.bundleFilePath);

    // This function will throw an error if certificate with specified ID is not present
    try {
      data.certificate = await getCertificate(data.apiKey, certificateId);
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
      // Certificate is already created, so we just need to pass validation
      // and download certificate file

      // We need to download new certificate bundle
      data.isBundleFilePresent = false;

      return {
        error: ERRORS.CERTIFICATE_IS_NOT_VALIDATED,
        data,
      };
    }

    if (data.certificate.status !== 'issued' || data.isExpiresSoon) {
      // Certificate is going to expire soon, or current certificate is not valid
      // we need to obtain a new one

      // We need to download new certificate bundle
      data.isBundleFilePresent = false;

      if (!data.isCsrFilePresent) {
        return {
          error: ERRORS.CSR_FILE_IS_NOT_PRESENT,
          data,
        };
      }

      data.csr = fs.readFileSync(data.csrFilePath, 'utf8');

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

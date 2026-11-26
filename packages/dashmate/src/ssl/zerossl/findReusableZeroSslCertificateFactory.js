import Certificate from './Certificate.js';
import { ERRORS, isSavedCertificateReusable } from './validateZeroSslCertificateFactory.js';

/**
 * How long advice waits for ZeroSSL before it is given without ZeroSSL's
 * answer. `start` refuses while it waits.
 */
const ANSWER_TIMEOUT_MS = 10000;

/**
 * @param {validateZeroSslCertificate} validateZeroSslCertificate
 * @return {findReusableZeroSslCertificate}
 */
export default function findReusableZeroSslCertificateFactory(validateZeroSslCertificate) {
  /**
   * Whether the ZeroSSL certificate a config saved can be installed without a
   * new one
   *
   * Decided by the validation `ssl obtain` runs, so advice and obtain agree.
   * Whether the gateway can serve the files on disk is a different question:
   * ZeroSSL saves the certificate ID before verification and the files are
   * written last, so an issued certificate can stand behind files that are
   * missing or cut short.
   *
   * @typedef {function} findReusableZeroSslCertificate
   * @param {Config} config
   * @return {Promise<{ isReusable: boolean|null, status: string|null, reason: string|null }>}
   *  isReusable is null when ZeroSSL could not be asked; status is ZeroSSL's
   *  for the certificate; reason says why it is not reusable or not known
   */
  async function findReusableZeroSslCertificate(config) {
    let result;

    try {
      result = await validateZeroSslCertificate(config, Certificate.EXPIRATION_LIMIT_DAYS, {
        signal: AbortSignal.timeout(ANSWER_TIMEOUT_MS),
      });
    } catch (e) {
      return { isReusable: null, status: null, reason: e.message };
    }

    const { error, data } = result;
    const status = data.certificate?.status ?? null;

    if (isSavedCertificateReusable(error)) {
      return { isReusable: true, status, reason: null };
    }

    const unknown = (reason) => ({ isReusable: null, status, reason });
    const notReusable = (reason) => ({ isReusable: false, status, reason });

    switch (error) {
      case ERRORS.API_KEY_IS_NOT_SET:
        return unknown('no ZeroSSL API key is set');
      case ERRORS.EXTERNAL_IP_IS_NOT_SET:
        return unknown('externalIp is not set');
      case ERRORS.ZERO_SSL_API_ERROR:
        return unknown(data.error.message);
      case ERRORS.CERTIFICATE_ID_IS_NOT_SET:
        return notReusable('no certificate ID is saved');
      case ERRORS.PRIVATE_KEY_IS_NOT_PRESENT:
        return notReusable('the private key it was requested with is not on this node');
      case ERRORS.EXTERNAL_IP_MISMATCH:
        return notReusable(`it was issued for ${data.certificate.common_name}, and externalIp is`
          + ` ${data.externalIp}`);
      default:
        // Not issued, or expiring within the days an obtain renews it.
        return notReusable(status === 'issued'
          ? `it expires on ${data.certificate.expires.toISOString().slice(0, 10)}`
          : `ZeroSSL reports it ${status}`);
    }
  }

  return findReusableZeroSslCertificate;
}

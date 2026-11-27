/**
 * Checks on the certificate the gateway serves that do not depend on who asks
 *
 * `ssl obtain` checks the pair before it enables TLS, and `update` checks it
 * again afterwards. Each check lives here once, so the two cannot accept and
 * refuse the same certificate. What each caller does with the answer stays
 * with the caller.
 */

/**
 * Whether a certificate verifies under its own public key, which is what
 * makes it self-signed
 *
 * Subject equal to issuer is only a naming convention: a self-signed
 * certificate may name any issuer it likes, and an authority can issue a
 * certificate whose issuer name repeats its subject.
 *
 * @param {crypto.X509Certificate} certificate
 * @return {boolean}
 */
export function isSelfSigned(certificate) {
  try {
    return certificate.verify(certificate.publicKey);
  } catch {
    return false;
  }
}

/**
 * Where a moment falls in a certificate's validity window
 *
 * Clients reject a certificate outside the window on either side. Nothing is
 * concluded about the clock: a fast local clock and a start date in the
 * future look the same from disk.
 *
 * @param {crypto.X509Certificate} certificate
 * @param {Date} [now]
 * @return {{ validFrom: Date, validTo: Date, isNotYetValid: boolean, isExpired: boolean }}
 */
export function readValidityWindow(certificate, now = new Date()) {
  const validFrom = new Date(certificate.validFrom);
  const validTo = new Date(certificate.validTo);

  return {
    validFrom,
    validTo,
    isNotYetValid: validFrom.getTime() > now.getTime(),
    isExpired: validTo.getTime() <= now.getTime(),
  };
}

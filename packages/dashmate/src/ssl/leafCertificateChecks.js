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
 * A date that does not parse gives an invalid Date, which compares as neither
 * before nor after now, so it would pass both window checks. Such a window is
 * reported as unreadable, and callers refuse it before reading the rest.
 *
 * @param {crypto.X509Certificate} certificate
 * @param {Date} [now]
 * @return {{ validFrom: Date, validTo: Date, isUnreadable: boolean,
 *   isNotYetValid: boolean, isExpired: boolean }}
 */
export function readValidityWindow(certificate, now = new Date()) {
  const validFrom = new Date(certificate.validFrom);
  const validTo = new Date(certificate.validTo);

  return {
    validFrom,
    validTo,
    isUnreadable: !Number.isFinite(validFrom.getTime()) || !Number.isFinite(validTo.getTime()),
    isNotYetValid: validFrom.getTime() > now.getTime(),
    isExpired: validTo.getTime() <= now.getTime(),
  };
}

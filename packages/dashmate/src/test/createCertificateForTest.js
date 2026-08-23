import forge from 'node-forge';

/**
 * Create a self-signed certificate with a chosen validity window and IP address.
 *
 * Certificates are generated when a test runs rather than committed, so a fixture cannot
 * expire and fail the suite on a date nobody chose. Built with node-forge rather than the
 * openssl binary because the flags needed to place a certificate in the past arrived in
 * OpenSSL 3.5, which is newer than the version on the CI image.
 *
 * @param {Object} [options]
 * @param {string} [options.ip] - placed in the subject alternative name and common name
 * @param {number} [options.days] - days from now the certificate expires, negative for expired
 * @return {{cert: string, key: string}} PEM encoded
 */
export default function createCertificateForTest({ ip = '127.0.0.1', days = 30 } = {}) {
  const keys = forge.pki.rsa.generateKeyPair(2048);
  const certificate = forge.pki.createCertificate();

  certificate.publicKey = keys.publicKey;
  certificate.serialNumber = '01';

  // Anchored to the expiry so an already-expired certificate still starts before it ends
  certificate.validity.notAfter = new Date(Date.now() + days * 24 * 60 * 60 * 1000);
  // Anchored to whichever of now and the expiry comes first, so the window
  // always starts in the past - as a real certificate's does. Anchoring to the
  // expiry alone put the start date in the future for anything valid longer
  // than the window itself.
  certificate.validity.notBefore = new Date(
    Math.min(Date.now(), certificate.validity.notAfter.getTime()) - 30 * 24 * 60 * 60 * 1000,
  );

  const attributes = [{ name: 'commonName', value: ip }];

  certificate.setSubject(attributes);
  certificate.setIssuer(attributes);
  certificate.setExtensions([
    { name: 'basicConstraints', cA: false },
    // Type 7 is an IP address. An evonode is identified by its address, not by a name.
    { name: 'subjectAltName', altNames: [{ type: 7, ip }] },
  ]);

  certificate.sign(keys.privateKey, forge.md.sha256.create());

  return {
    cert: forge.pki.certificateToPem(certificate),
    key: forge.pki.privateKeyToPem(keys.privateKey),
  };
}

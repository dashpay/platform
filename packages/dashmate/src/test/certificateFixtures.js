import crypto from 'node:crypto';
import forge from 'node-forge';

/**
 * Build the certificate shapes the gateway certificate checks have to tell apart.
 *
 * Certificates are generated when a test runs rather than committed, so a
 * fixture cannot expire and fail the suite on a date nobody chose. node-forge
 * rather than the openssl binary because placing a certificate in the past
 * needs flags that arrived in OpenSSL 3.5, which is newer than the CI image.
 */

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * @param {Object} name
 * @return {Object[]} node-forge subject attributes
 */
function toAttributes({ commonName, organizationName } = {}) {
  const attributes = [];

  if (organizationName !== undefined) {
    attributes.push({ name: 'organizationName', value: organizationName });
  }

  if (commonName !== undefined) {
    attributes.push({ name: 'commonName', value: commonName });
  }

  return attributes;
}

/**
 * @param {Buffer} der
 * @return {string}
 */
function toPem(der) {
  const body = der.toString('base64').match(/.{1,64}/g).join('\n');

  return `-----BEGIN CERTIFICATE-----\n${body}\n-----END CERTIFICATE-----\n`;
}

/**
 * Issue a certificate, optionally signed by another one.
 *
 * @param {Object} [options]
 * @param {Object} [options.subject] - {commonName, organizationName}, both optional
 * @param {Object} [options.issuer] - the issuing authority, self-signed when absent
 * @param {string} [options.ip] - placed in the subject alternative name
 * @param {number} [options.days] - days from now it expires, negative for expired
 * @param {boolean} [options.ca]
 * @param {Object} [options.keys] - reuse an existing node-forge key pair
 * @return {{pem: string, keyPem: string, keys: Object, certificate: Object,
 *   subject: Object}}
 */
export function issueCertificate({
  subject = { commonName: '1.2.3.4' },
  issuer,
  ip,
  days = 30,
  ca = false,
  keys = forge.pki.rsa.generateKeyPair(2048),
} = {}) {
  const certificate = forge.pki.createCertificate();

  certificate.publicKey = keys.publicKey;
  certificate.serialNumber = '01';

  // Anchored to the expiry so an already-expired certificate still starts
  // before it ends.
  certificate.validity.notAfter = new Date(Date.now() + days * DAY_MS);
  certificate.validity.notBefore = new Date(
    certificate.validity.notAfter.getTime() - 90 * DAY_MS,
  );

  certificate.setSubject(toAttributes(subject));
  certificate.setIssuer(toAttributes(issuer ? issuer.subject : subject));

  const extensions = [{ name: 'basicConstraints', cA: ca }];

  if (ip) {
    // Type 7 is an IP address. An evonode is identified by its address.
    extensions.push({ name: 'subjectAltName', altNames: [{ type: 7, ip }] });
  }

  certificate.setExtensions(extensions);

  certificate.sign(
    issuer ? issuer.keys.privateKey : keys.privateKey,
    forge.md.sha256.create(),
  );

  return {
    pem: forge.pki.certificateToPem(certificate),
    keyPem: forge.pki.privateKeyToPem(keys.privateKey),
    keys,
    certificate,
    subject,
  };
}

/**
 * A leaf, the intermediate that signed it and the self-signed root above that -
 * the ordinary shape of a publicly trusted bundle.
 *
 * @param {Object} [options]
 * @param {string} [options.ip]
 * @param {number} [options.days]
 * @param {string} [options.organizationName] - the issuing CA's organisation
 * @return {{leaf: Object, intermediate: Object, root: Object}}
 */
export function issueChain({
  ip = '1.2.3.4',
  days = 6,
  organizationName = "Let's Encrypt",
} = {}) {
  const root = issueCertificate({
    subject: { organizationName, commonName: 'Test Root X1' },
    days: 3650,
    ca: true,
  });

  const intermediate = issueCertificate({
    subject: { organizationName, commonName: 'R11' },
    issuer: root,
    days: 1800,
    ca: true,
  });

  // lego passes --disable-cn for an IP certificate, so the address is only ever
  // in the subject alternative name.
  const leaf = issueCertificate({
    subject: {},
    issuer: intermediate,
    ip,
    days,
  });

  return { leaf, intermediate, root };
}

/**
 * Replace a certificate's public key with an Ed25519 one and re-sign it.
 *
 * node-forge cannot build a certificate around a key type it does not
 * implement, but the certificate only has to carry the key - the signature over
 * it is still the issuer's RSA one, which is a legitimate combination. This is
 * what proves the leaf is selected by comparing key material rather than by an
 * RSA-only signature test.
 *
 * @param {Object} issuer - the authority whose key signs the result
 * @param {Object} [options]
 * @param {string} [options.ip]
 * @param {number} [options.days]
 * @return {{pem: string, keyPem: string}}
 */
export function issueEd25519Certificate(issuer, { ip = '1.2.3.4', days = 6 } = {}) {
  const { privateKey, publicKey } = crypto.generateKeyPairSync('ed25519');
  const spki = publicKey.export({ type: 'spki', format: 'der' });

  const template = issueCertificate({
    subject: {}, issuer, ip, days,
  });

  const asn1 = forge.pki.certificateToAsn1(template.certificate);
  const tbsCertificate = asn1.value[0];

  // TBSCertificate ::= SEQUENCE { version [0], serialNumber, signature,
  // issuer, validity, subject, subjectPublicKeyInfo, ... }
  tbsCertificate.value[6] = forge.asn1.fromDer(
    forge.util.createBuffer(Buffer.from(spki).toString('binary')),
  );

  const tbsDer = Buffer.from(forge.asn1.toDer(tbsCertificate).getBytes(), 'binary');
  const digest = forge.md.sha256.create();
  digest.update(tbsDer.toString('binary'));

  asn1.value[2] = forge.asn1.create(
    forge.asn1.Class.UNIVERSAL,
    forge.asn1.Type.BITSTRING,
    false,
    String.fromCharCode(0) + issuer.keys.privateKey.sign(digest),
  );

  return {
    pem: toPem(Buffer.from(forge.asn1.toDer(asn1).getBytes(), 'binary')),
    keyPem: privateKey.export({ type: 'pkcs8', format: 'pem' }),
  };
}

/**
 * @param {Object} keys - a node-forge key pair
 * @param {string} passphrase
 * @return {string} PEM
 */
export function encryptPrivateKey(keys, passphrase) {
  return forge.pki.encryptRsaPrivateKey(keys.privateKey, passphrase);
}

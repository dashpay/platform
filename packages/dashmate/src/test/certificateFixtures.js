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
 * @param {number} [options.startsInDays] - days from now it becomes valid
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
  startsInDays,
  ca = false,
  keys = forge.pki.rsa.generateKeyPair(2048),
} = {}) {
  const certificate = forge.pki.createCertificate();

  certificate.publicKey = keys.publicKey;
  certificate.serialNumber = '01';

  // Anchored to the expiry so an already-expired certificate still starts
  // before it ends.
  certificate.validity.notAfter = new Date(Date.now() + days * DAY_MS);
  certificate.validity.notBefore = startsInDays === undefined
    ? new Date(certificate.validity.notAfter.getTime() - 90 * DAY_MS)
    : new Date(Date.now() + startsInDays * DAY_MS);

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

/**
 * @param {number} tag
 * @param {...Buffer} contents
 * @return {Buffer} the DER element
 */
function encodeElement(tag, ...contents) {
  const content = Buffer.concat(contents);
  const lengthBytes = [];

  for (let rest = content.length; rest > 0; rest = Math.floor(rest / 256)) {
    lengthBytes.unshift(rest % 256);
  }

  const length = content.length < 0x80
    ? Buffer.from([content.length])
    : Buffer.from([0x80 + lengthBytes.length, ...lengthBytes]);

  return Buffer.concat([Buffer.from([tag]), length, content]);
}

/**
 * @param {string} oid
 * @return {Buffer} the DER OBJECT IDENTIFIER
 */
function encodeOid(oid) {
  return encodeElement(0x06, Buffer.from(forge.asn1.oidToDer(oid).getBytes(), 'binary'));
}

/**
 * Make a certificate request for a key of any type, the way OpenSSL makes one
 *
 * node-forge makes requests for RSA keys only, and re-encodes what it
 * parses, so the request is written out byte by byte here and Node signs it.
 *
 * @param {string} privateKeyPem - the key the request carries
 * @param {Object} [options]
 * @param {string} [options.commonName]
 * @param {string} [options.signingKeyPem] - signs the request, the key it
 *  carries when absent
 * @param {Buffer} [options.version] - the version element, INTEGER 0 when
 *  absent
 * @param {Buffer} [options.attributes] - the content of the attributes
 * @param {Object} [options.algorithm] - sha256 with RSA or ECDSA when absent
 * @param {string} options.algorithm.oid
 * @param {string} options.algorithm.hash - as Node names it
 * @param {Buffer} [options.algorithm.parameters] - NULL for RSA, none for EC
 *  when absent
 * @return {string} PEM
 */
export function createCertificateRequest(privateKeyPem, {
  commonName = '1.2.3.4',
  signingKeyPem = privateKeyPem,
  version = Buffer.from([0x02, 0x01, 0x00]),
  attributes = Buffer.alloc(0),
  algorithm,
} = {}) {
  const spki = crypto.createPublicKey(crypto.createPrivateKey(privateKeyPem))
    .export({ type: 'spki', format: 'der' });
  const signingKey = crypto.createPrivateKey(signingKeyPem);
  const isRsa = signingKey.asymmetricKeyType === 'rsa';

  const {
    oid = isRsa ? '1.2.840.113549.1.1.11' : '1.2.840.10045.4.3.2',
    hash = 'sha256',
    parameters = isRsa ? Buffer.from([0x05, 0x00]) : Buffer.alloc(0),
  } = algorithm ?? {};

  // Name ::= SEQUENCE OF SET OF { type, value }, the value a UTF8String
  const subject = encodeElement(0x30, encodeElement(0x31, encodeElement(
    0x30,
    encodeOid('2.5.4.3'),
    encodeElement(0x0c, Buffer.from(commonName, 'utf8')),
  )));

  // CertificationRequestInfo ::= SEQUENCE { version, subject,
  // subjectPKInfo, attributes [0] }
  const info = encodeElement(0x30, version, subject, spki, encodeElement(0xa0, attributes));

  const signature = crypto.sign(hash, info, signingKey);

  const request = encodeElement(
    0x30,
    info,
    encodeElement(0x30, encodeOid(oid), parameters),
    encodeElement(0x03, Buffer.from([0]), signature),
  );

  return forge.pem.encode({ type: 'CERTIFICATE REQUEST', body: request.toString('binary') });
}

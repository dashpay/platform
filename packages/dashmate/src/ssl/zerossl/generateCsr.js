import crypto from 'crypto';
import forge from 'node-forge';

const COMMON_NAME_OID = '2.5.4.3';
const ECDSA_WITH_SHA256_OID = '1.2.840.10045.4.3.2';

const TAGS = {
  INTEGER: 0x02,
  BIT_STRING: 0x03,
  OID: 0x06,
  PRINTABLE_STRING: 0x13,
  SEQUENCE: 0x30,
  SET: 0x31,
  ATTRIBUTES: 0xa0,
};

/**
 * @param {number} tag
 * @param {...Buffer} contents
 * @return {Buffer} the DER element, its length in the shortest form
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
  return encodeElement(TAGS.OID, Buffer.from(forge.asn1.oidToDer(oid).getBytes(), 'binary'));
}

/**
 * Write a request for an EC key
 *
 * node-forge implements RSA only, so the request is written out in DER here
 * and Node signs it. It holds what node-forge writes into an RSA request:
 * version 0, the address as the common name in a PrintableString, the public
 * key and no attributes. It is signed with ECDSA and SHA-256, as OpenSSL
 * signs one by default.
 *
 * @param {crypto.KeyObject} privateKey
 * @param {string} externalIp
 * @return {string} PEM
 */
function generateEcCsr(privateKey, externalIp) {
  const subjectPublicKeyInfo = crypto.createPublicKey(privateKey)
    .export({ type: 'spki', format: 'der' });

  // Name ::= SEQUENCE OF SET OF { type, value }
  const subject = encodeElement(TAGS.SEQUENCE, encodeElement(TAGS.SET, encodeElement(
    TAGS.SEQUENCE,
    encodeOid(COMMON_NAME_OID),
    encodeElement(TAGS.PRINTABLE_STRING, Buffer.from(externalIp, 'ascii')),
  )));

  // CertificationRequestInfo ::= SEQUENCE { version, subject,
  // subjectPKInfo, attributes [0] }
  const info = encodeElement(
    TAGS.SEQUENCE,
    encodeElement(TAGS.INTEGER, Buffer.from([0])),
    subject,
    subjectPublicKeyInfo,
    encodeElement(TAGS.ATTRIBUTES),
  );

  // DER-encoded, as a request carries it
  const signature = crypto.sign('sha256', info, privateKey);

  // CertificationRequest ::= SEQUENCE { certificationRequestInfo,
  // signatureAlgorithm, signature BIT STRING }, ECDSA taking no parameters
  const request = encodeElement(
    TAGS.SEQUENCE,
    info,
    encodeElement(TAGS.SEQUENCE, encodeOid(ECDSA_WITH_SHA256_OID)),
    encodeElement(TAGS.BIT_STRING, Buffer.from([0]), signature),
  );

  return forge.pem.encode({ type: 'CERTIFICATE REQUEST', body: request.toString('binary') });
}

/**
 * Generate a CSR
 *
 * For an RSA key, the one dashmate generates, or an EC key: ZeroSSL issues
 * certificates for those only, and the key on disk may be either when an
 * issued certificate is installed and its request written again.
 *
 * @typedef {generateCsr}
 * @param {Object} keyPair
 * @param {string} [keyPair.publicKey]
 * @param {string} [keyPair.privateKey]
 * @param {string} externalIp
 * @return {Promise<string>}
 */
export default async function generateCsr(
  keyPair,
  externalIp,
) {
  const privateKey = crypto.createPrivateKey(keyPair.privateKey);

  if (privateKey.asymmetricKeyType === 'ec') {
    return generateEcCsr(privateKey, externalIp);
  }

  if (privateKey.asymmetricKeyType !== 'rsa') {
    throw new Error(`Cannot write a certificate request for an ${privateKey.asymmetricKeyType} key:`
      + ' dashmate writes them for RSA and EC keys, the ones ZeroSSL issues certificates for');
  }

  const csr = forge.pki.createCertificationRequest();
  csr.publicKey = forge.pki.publicKeyFromPem(keyPair.publicKey);

  csr.setSubject([
    { name: 'commonName', value: externalIp },
  ]);

  csr.sign(forge.pki.privateKeyFromPem(keyPair.privateKey));

  return forge.pki.certificationRequestToPem(csr);
}

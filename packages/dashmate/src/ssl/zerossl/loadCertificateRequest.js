import crypto from 'crypto';
import forge from 'node-forge';

const TAGS = {
  INTEGER: 0x02,
  BIT_STRING: 0x03,
  OID: 0x06,
  SEQUENCE: 0x30,
  ATTRIBUTES: 0xa0,
};

/**
 * RSA's PKCS #1 v1.5 signatures take a NULL parameter, which some writers
 * leave out, and ECDSA's take none
 */
const RSA_PARAMETERS = [Buffer.from([0x05, 0x00]), Buffer.alloc(0)];
const ECDSA_PARAMETERS = [Buffer.alloc(0)];

/**
 * The signature algorithms of RSA and EC requests: the hash each signs with,
 * the key type it needs, and the parameters it takes
 */
const SIGNATURE_ALGORITHMS = new Map([
  ['1.2.840.113549.1.1.5', 'sha1'],
  ['1.2.840.113549.1.1.14', 'sha224'],
  ['1.2.840.113549.1.1.11', 'sha256'],
  ['1.2.840.113549.1.1.12', 'sha384'],
  ['1.2.840.113549.1.1.13', 'sha512'],
].map(([oid, hash]) => [oid, { hash, keyType: 'rsa', parameters: RSA_PARAMETERS }]).concat([
  ['1.2.840.10045.4.1', 'sha1'],
  ['1.2.840.10045.4.3.1', 'sha224'],
  ['1.2.840.10045.4.3.2', 'sha256'],
  ['1.2.840.10045.4.3.3', 'sha384'],
  ['1.2.840.10045.4.3.4', 'sha512'],
].map(([oid, hash]) => [oid, { hash, keyType: 'ec', parameters: ECDSA_PARAMETERS }]))
  // Keyed by the OID's DER content, so only its one encoding matches
  .map(([oid, scheme]) => [forge.asn1.oidToDer(oid).toHex(), scheme]));

const REQUEST_PEM_TYPES = ['CERTIFICATE REQUEST', 'NEW CERTIFICATE REQUEST'];

/**
 * Read the DER element at an offset: a single-byte tag and a definite
 * length in its shortest form, ending within the given end
 *
 * @param {Buffer} der
 * @param {number} offset
 * @param {number} end
 * @param {number} tag - the tag it has to have
 * @return {{ start: number, contentStart: number, end: number }}
 * @throws {Error} when the element is not there
 */
function readElement(der, offset, end, tag) {
  if (offset + 2 > end || der[offset] !== tag) {
    throw new Error('unexpected element');
  }

  let length = der[offset + 1];
  let contentStart = offset + 2;

  if (length >= 0x80) {
    const count = length - 0x80;

    if (count === 0 || count > 4 || contentStart + count > end || der[contentStart] === 0) {
      throw new Error('unexpected length');
    }

    length = der.readUIntBE(contentStart, count);
    contentStart += count;

    if (length < 0x80) {
      throw new Error('unexpected length');
    }
  }

  if (contentStart + length > end) {
    throw new Error('element is cut short');
  }

  return { start: offset, contentStart, end: contentStart + length };
}

/**
 * Load the certificate request a PEM opens with, and check its signature
 *
 * The request is read as the DER bytes it is, and its signature is checked
 * over those bytes: they are what ZeroSSL gets. node-forge loads requests
 * for RSA keys only and encodes what it parses again, so it only decodes the
 * PEM here. Node loads the public key and checks the signature, for RSA and
 * EC keys alike. Attributes and the subject are left to ZeroSSL.
 *
 * @param {string} pem
 * @return {{ publicKey: crypto.KeyObject, pem: string }|null} its key and the
 *  request alone, or null when it does not load, or its own key did not sign
 *  it
 */
export default function loadCertificateRequest(pem) {
  try {
    const [block] = forge.pem.decode(pem);

    if (!REQUEST_PEM_TYPES.includes(block?.type)) {
      return null;
    }

    const der = Buffer.from(block.body, 'binary');

    // CertificationRequest ::= SEQUENCE { certificationRequestInfo,
    // signatureAlgorithm, signature BIT STRING }
    const request = readElement(der, 0, der.length, TAGS.SEQUENCE);
    const info = readElement(der, request.contentStart, request.end, TAGS.SEQUENCE);
    const algorithm = readElement(der, info.end, request.end, TAGS.SEQUENCE);
    const signature = readElement(der, algorithm.end, request.end, TAGS.BIT_STRING);

    if (request.end !== der.length || signature.end !== request.end) {
      return null;
    }

    // CertificationRequestInfo ::= SEQUENCE { version INTEGER (0), subject,
    // subjectPKInfo, attributes [0] }
    const version = readElement(der, info.contentStart, info.end, TAGS.INTEGER);
    const subject = readElement(der, version.end, info.end, TAGS.SEQUENCE);
    const subjectPublicKeyInfo = readElement(der, subject.end, info.end, TAGS.SEQUENCE);
    const attributes = readElement(der, subjectPublicKeyInfo.end, info.end, TAGS.ATTRIBUTES);

    if (attributes.end !== info.end
      || !der.subarray(version.contentStart, version.end).equals(Buffer.from([0]))) {
      return null;
    }

    // AlgorithmIdentifier ::= SEQUENCE { algorithm OID, parameters }
    const oid = readElement(der, algorithm.contentStart, algorithm.end, TAGS.OID);
    const scheme = SIGNATURE_ALGORITHMS.get(der.subarray(oid.contentStart, oid.end).toString('hex'));
    const parameters = der.subarray(oid.end, algorithm.end);

    if (scheme === undefined || !scheme.parameters.some((allowed) => allowed.equals(parameters))) {
      return null;
    }

    // The signature is whole bytes
    const signatureBits = der.subarray(signature.contentStart, signature.end);

    if (signatureBits.length < 2 || signatureBits[0] !== 0) {
      return null;
    }

    const publicKey = crypto.createPublicKey({
      key: der.subarray(subjectPublicKeyInfo.start, subjectPublicKeyInfo.end),
      format: 'der',
      type: 'spki',
    });

    if (publicKey.asymmetricKeyType !== scheme.keyType) {
      return null;
    }

    const isSigned = crypto.verify(
      scheme.hash,
      der.subarray(info.start, info.end),
      publicKey,
      signatureBits.subarray(1),
    );

    if (!isSigned) {
      return null;
    }

    return {
      publicKey,
      pem: forge.pem.encode({ type: 'CERTIFICATE REQUEST', body: block.body }),
    };
  } catch {
    return null;
  }
}

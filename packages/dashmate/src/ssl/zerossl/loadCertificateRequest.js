import crypto from 'crypto';
import forge from 'node-forge';

/**
 * The signature algorithms of RSA and EC requests, by OID: the hash each
 * signs with and the key type it needs
 */
const SIGNATURE_ALGORITHMS = {
  '1.2.840.113549.1.1.5': { hash: 'sha1', keyType: 'rsa' },
  '1.2.840.113549.1.1.14': { hash: 'sha224', keyType: 'rsa' },
  '1.2.840.113549.1.1.11': { hash: 'sha256', keyType: 'rsa' },
  '1.2.840.113549.1.1.12': { hash: 'sha384', keyType: 'rsa' },
  '1.2.840.113549.1.1.13': { hash: 'sha512', keyType: 'rsa' },
  '1.2.840.10045.4.1': { hash: 'sha1', keyType: 'ec' },
  '1.2.840.10045.4.3.1': { hash: 'sha224', keyType: 'ec' },
  '1.2.840.10045.4.3.2': { hash: 'sha256', keyType: 'ec' },
  '1.2.840.10045.4.3.3': { hash: 'sha384', keyType: 'ec' },
  '1.2.840.10045.4.3.4': { hash: 'sha512', keyType: 'ec' },
};

const REQUEST_PEM_TYPES = ['CERTIFICATE REQUEST', 'NEW CERTIFICATE REQUEST'];

/**
 * @param {Object} node - node-forge ASN.1
 * @return {Buffer}
 */
function toDer(node) {
  return Buffer.from(forge.asn1.toDer(node).getBytes(), 'binary');
}

/**
 * Load the certificate request a PEM opens with, and check its signature
 *
 * node-forge loads requests for RSA keys only, so it only splits the request
 * into its ASN.1 parts here. Node loads the public key and checks the
 * signature, for RSA and EC keys alike.
 *
 * @param {string} pem
 * @return {{ publicKey: crypto.KeyObject }|null} null when the request does
 *  not load, or its own key did not sign it
 */
export default function loadCertificateRequest(pem) {
  try {
    const [block] = forge.pem.decode(pem);

    if (!REQUEST_PEM_TYPES.includes(block?.type)) {
      return null;
    }

    // CertificationRequest ::= SEQUENCE { certificationRequestInfo,
    // signatureAlgorithm, signature BIT STRING }, and the info is
    // SEQUENCE { version, subject, subjectPKInfo, attributes [0] }
    const request = forge.asn1.fromDer(block.body, { decodeBitStrings: false });
    const [info, algorithm, signature] = request.value;

    if (request.value.length !== 3
      || signature.type !== forge.asn1.Type.BITSTRING
      // The signature is whole bytes
      || signature.value.charCodeAt(0) !== 0) {
      return null;
    }

    const publicKey = crypto.createPublicKey({
      key: toDer(info.value[2]),
      format: 'der',
      type: 'spki',
    });

    const scheme = SIGNATURE_ALGORITHMS[forge.asn1.derToOid(algorithm.value[0].value)];

    if (scheme === undefined || publicKey.asymmetricKeyType !== scheme.keyType) {
      return null;
    }

    const isSigned = crypto.verify(
      scheme.hash,
      toDer(info),
      publicKey,
      Buffer.from(signature.value.slice(1), 'binary'),
    );

    return isSigned ? { publicKey } : null;
  } catch {
    return null;
  }
}

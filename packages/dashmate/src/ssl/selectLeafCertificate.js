import crypto from 'node:crypto';

export const LEAF_SELECTION_ERRORS = {
  KEY_UNUSABLE: 'KEY_UNUSABLE',
  BUNDLE_UNREADABLE: 'BUNDLE_UNREADABLE',
  KEY_MISMATCH: 'KEY_MISMATCH',
};

const PEM_CERTIFICATE = /-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g;

/**
 * A key protected by a passphrase is detected from the PEM rather than by
 * asking OpenSSL, which can go looking for a terminal to ask on.
 */
const ENCRYPTED_KEY = /-----BEGIN ENCRYPTED PRIVATE KEY-----|^\s*Proc-Type:\s*4,ENCRYPTED/m;

/**
 * @param {crypto.KeyObject} publicKey
 * @return {Buffer|null}
 */
function exportSubjectPublicKeyInfo(publicKey) {
  try {
    return publicKey.export({ type: 'spki', format: 'der' });
  } catch {
    return null;
  }
}

/**
 * Find the certificate in a bundle that belongs to a private key.
 *
 * The leaf is the block whose public key material is the key's own. That single
 * rule does three jobs: it finds the leaf whichever way round the bundle is
 * written, it is itself the pairing check, and comparing key material rather
 * than verifying an RSA signature works for every key type an authority might
 * issue.
 *
 * Selecting by position gets one bundle order wrong, and self-sign testing
 * every block to find the leaf rejects any chain that carries its own root -
 * which an ordinary publicly trusted bundle does.
 *
 * @param {string} bundlePem
 * @param {string} privateKeyPem
 * @return {{leaf: crypto.X509Certificate}|{error: string, detail: string}}
 */
export default function selectLeafCertificate(bundlePem, privateKeyPem) {
  if (ENCRYPTED_KEY.test(privateKeyPem)) {
    return {
      error: LEAF_SELECTION_ERRORS.KEY_UNUSABLE,
      detail: 'it is protected by a passphrase, and the gateway has no way to be given one',
    };
  }

  let subjectPublicKeyInfo;
  try {
    subjectPublicKeyInfo = exportSubjectPublicKeyInfo(
      crypto.createPublicKey(crypto.createPrivateKey(privateKeyPem)),
    );
  } catch (e) {
    return { error: LEAF_SELECTION_ERRORS.KEY_UNUSABLE, detail: e.message };
  }

  if (subjectPublicKeyInfo === null) {
    return { error: LEAF_SELECTION_ERRORS.KEY_UNUSABLE, detail: 'its key material could not be read' };
  }

  const certificates = (bundlePem.match(PEM_CERTIFICATE) ?? [])
    .map((block) => {
      try {
        return new crypto.X509Certificate(block);
      } catch {
        // A block that will not parse is skipped rather than failing the whole
        // bundle, which may hold comments or a stray key.
        return null;
      }
    })
    .filter(Boolean);

  if (certificates.length === 0) {
    return { error: LEAF_SELECTION_ERRORS.BUNDLE_UNREADABLE, detail: 'it holds no certificate' };
  }

  const leaf = certificates.find((certificate) => {
    const spki = exportSubjectPublicKeyInfo(certificate.publicKey);

    return spki !== null && spki.equals(subjectPublicKeyInfo);
  });

  if (!leaf) {
    return {
      error: LEAF_SELECTION_ERRORS.KEY_MISMATCH,
      detail: 'no certificate in the bundle belongs to the private key',
    };
  }

  return { leaf };
}

import crypto from 'node:crypto';

export const LEAF_SELECTION_ERRORS = {
  KEY_UNUSABLE: 'KEY_UNUSABLE',
  BUNDLE_UNREADABLE: 'BUNDLE_UNREADABLE',
  KEY_MISMATCH: 'KEY_MISMATCH',
  BUNDLE_ORDER: 'BUNDLE_ORDER',
};

/**
 * Both delimiters must be a line of their own. Matched as substrings they can
 * be read out of the middle of a mangled line - a stray prefix, indentation, an
 * extra hyphen - and the gateway refuses such a file. The optional carriage
 * return keeps a bundle written with Windows line endings valid, which the
 * gateway loads without complaint.
 */
const PEM_CERTIFICATE = /^-----BEGIN CERTIFICATE-----\r?$[\s\S]*?^-----END CERTIFICATE-----\r?$/gm;

/**
 * Deliberately unanchored, unlike the block match above. Anything that looks
 * like an opening counts as one, so a marker the block match rightly refused -
 * because its line carries something else, or because it never closes - still
 * registers, and the totals disagree.
 *
 * Only openings are counted. A stray END with no BEGIN is loaded by the gateway
 * without complaint, so treating unpaired delimiters symmetrically would reject
 * a bundle that works.
 */
const BEGIN_CERTIFICATE = /-----BEGIN CERTIFICATE-----/g;

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
 * The leaf is the block whose public key material is the key's own. Comparing
 * key material is both how the leaf is recognised and the pairing check itself,
 * and unlike verifying an RSA signature it works for every key type an
 * authority might issue. Self-sign testing every block to find the leaf would
 * instead reject any chain carrying its own root, which an ordinary publicly
 * trusted bundle does.
 *
 * Identifying it that way does not make its position free. Envoy reads the
 * chain file in order and serves the first block as the leaf, so a bundle
 * written the other way round is broken at the gateway however well its
 * contents pair up. The key's certificate is therefore required to be the first
 * block as well as to exist.
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

  const blocks = bundlePem.match(PEM_CERTIFICATE) ?? [];
  const begins = (bundlePem.match(BEGIN_CERTIFICATE) ?? []).length;

  if (begins !== blocks.length) {
    return {
      error: LEAF_SELECTION_ERRORS.BUNDLE_UNREADABLE,
      detail: `it opens ${begins} certificate(s) but only ${blocks.length} are well formed,`
        + ' so at least one block is truncated or its delimiters are damaged',
    };
  }

  if (blocks.length === 0) {
    return { error: LEAF_SELECTION_ERRORS.BUNDLE_UNREADABLE, detail: 'it holds no certificate' };
  }

  const certificates = [];
  for (let position = 0; position < blocks.length; position += 1) {
    try {
      certificates.push(new crypto.X509Certificate(blocks[position]));
    } catch (e) {
      // Not skipped. The gateway loads this same file, so a block it will choke
      // on is a problem with the bundle even when a usable leaf sits beside it.
      return {
        error: LEAF_SELECTION_ERRORS.BUNDLE_UNREADABLE,
        detail: `certificate ${position + 1} of ${blocks.length} could not be parsed: ${e.message}`,
      };
    }
  }

  const position = certificates.findIndex((certificate) => {
    const spki = exportSubjectPublicKeyInfo(certificate.publicKey);

    return spki !== null && spki.equals(subjectPublicKeyInfo);
  });

  if (position === -1) {
    return {
      error: LEAF_SELECTION_ERRORS.KEY_MISMATCH,
      detail: 'no certificate in the bundle belongs to the private key',
    };
  }

  if (position !== 0) {
    return {
      error: LEAF_SELECTION_ERRORS.BUNDLE_ORDER,
      detail: `the certificate belonging to the private key is block ${position + 1}`
        + ` of ${certificates.length}, and the gateway serves the first block as the leaf`,
    };
  }

  return { leaf: certificates[0] };
}

import validateFileExists from './validateFileExists.js';
import containsPrivateKey from '../../../ssl/containsPrivateKey.js';
import readRegularFile from '../../../util/readRegularFile.js';
import selectLeafCertificate, { LEAF_SELECTION_ERRORS } from '../../../ssl/selectLeafCertificate.js';
import { readValidityWindow } from '../../../ssl/leafCertificateChecks.js';

/**
 * @param {string} filePath
 * @return {{ content: Buffer, stats: fs.Stats }}
 */
function readCertificateFile(filePath) {
  const file = readRegularFile(filePath);

  if (file === null) {
    throw new Error('the certificate and key must be regular files');
  }

  return file;
}

/**
 * Check that the gateway can serve a certificate chain with its private key
 *
 * The check is the one the install applies to what it wrote, so a pair that
 * would fail there, a damaged block after the leaf included, fails here. A
 * chain that holds a private key is refused too, and so is a certificate
 * outside its validity window: clients refuse it on either side, and
 * `update` refuses to go on with it.
 *
 * @param {Object} pair
 * @param {Buffer} pair.certificateFile
 * @param {Buffer} pair.privateKeyFile
 * @throws {Error} saying why the pair is refused
 */
export function checkCertificatePair({ certificateFile, privateKeyFile }) {
  // The chain is installed as given and readable by other users, while the
  // key file is kept to its owner, so a key in the chain would be a second,
  // exposed copy. The bytes are installed as checked, so the chain is
  // refused rather than stripped of it.
  if (containsPrivateKey(certificateFile.toString('latin1'))) {
    throw new Error('the certificate chain file holds a private key: give only the certificates'
      + ' in it, and the key in its own file');
  }

  // Decoded as the install decodes what it wrote when it checks it again.
  const { error, detail, leaf } = selectLeafCertificate(
    certificateFile.toString('utf8'),
    privateKeyFile.toString('utf8'),
  );

  if (error === LEAF_SELECTION_ERRORS.KEY_MISMATCH) {
    throw new Error('The certificate and private key do not match');
  }

  if (error) {
    throw new Error(`The certificate and private key cannot be used: ${detail}`);
  }

  const validity = readValidityWindow(leaf);

  if (validity.isNotYetValid) {
    throw new Error(`The certificate is not valid until ${leaf.validFrom}`);
  }

  if (validity.isExpired) {
    throw new Error(`The certificate expired on ${leaf.validTo}`);
  }
}

/**
 * Read a certificate chain and its private key, each once, and check that
 * the gateway can serve them
 *
 * The contents returned are the bytes checked: a caller that installs them
 * later installs exactly what passed, byte for byte, even if a file was
 * replaced meanwhile. The key file's stats are the ones of the file read.
 * They are checked by checkCertificatePair.
 *
 * Shared by the prompt and by the flags that answer it, so an answer given on
 * the command line is held to what the prompt would accept.
 *
 * @param {Object} paths
 * @param {string} paths.chainFilePath
 * @param {string} paths.privateFilePath
 * @return {{ certificateFile: Buffer, privateKeyFile: Buffer, privateKeyStats: fs.Stats }}
 * @throws {Error} saying why the pair is refused
 */
export function readCertificateFilePair({ chainFilePath, privateFilePath }) {
  if (!validateFileExists(chainFilePath)) {
    throw new Error('certificate chain file path is not valid');
  }

  if (!validateFileExists(privateFilePath)) {
    throw new Error('certificate key file path is not valid');
  }

  if (chainFilePath === privateFilePath) {
    throw new Error('the same path for both files');
  }

  const { content: certificateFile } = readCertificateFile(chainFilePath);
  const { content: privateKeyFile, stats: privateKeyStats } = readCertificateFile(privateFilePath);

  checkCertificatePair({ certificateFile, privateKeyFile });

  return { certificateFile, privateKeyFile, privateKeyStats };
}

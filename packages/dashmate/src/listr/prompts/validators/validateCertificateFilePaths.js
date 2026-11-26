import fs from 'fs';
import validateFileExists from './validateFileExists.js';
import selectLeafCertificate, { LEAF_SELECTION_ERRORS } from '../../../ssl/selectLeafCertificate.js';

/**
 * Read a file once, through a descriptor checked to be a regular file's
 *
 * Opening without blocking keeps a FIFO from waiting for a writer, and the
 * check is on what was opened, so the path cannot be swapped in between.
 *
 * @param {string} filePath
 * @return {{ content: Buffer, stats: fs.Stats }}
 */
function readRegularFile(filePath) {
  // eslint-disable-next-line no-bitwise
  const fd = fs.openSync(filePath, fs.constants.O_RDONLY | fs.constants.O_NONBLOCK);

  try {
    const stats = fs.fstatSync(fd);

    if (!stats.isFile()) {
      throw new Error('the certificate and key must be regular files');
    }

    return { content: fs.readFileSync(fd), stats };
  } finally {
    fs.closeSync(fd);
  }
}

/**
 * Read a certificate chain and its private key, each once, and check that
 * the gateway can serve them
 *
 * The contents returned are the bytes checked: a caller that installs them
 * later installs exactly what passed, byte for byte, even if a file was
 * replaced meanwhile. The key file's stats are the ones of the file read.
 * The check is the one the install applies to what it wrote, so a pair that
 * would fail there, a damaged block after the leaf included, fails here.
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

  const { content: certificateFile } = readRegularFile(chainFilePath);
  const { content: privateKeyFile, stats: privateKeyStats } = readRegularFile(privateFilePath);

  // Decoded as the install decodes what it wrote when it checks it again.
  const { error, detail } = selectLeafCertificate(
    certificateFile.toString('utf8'),
    privateKeyFile.toString('utf8'),
  );

  if (error === LEAF_SELECTION_ERRORS.KEY_MISMATCH) {
    throw new Error('The certificate and private key do not match');
  }

  if (error) {
    throw new Error(`The certificate and private key cannot be used: ${detail}`);
  }

  return { certificateFile, privateKeyFile, privateKeyStats };
}

import fs from 'fs';
import validateFileExists from './validateFileExists.js';
import validateSslCertificateFiles from './validateSslCertificateFiles.js';

/**
 * Check a certificate chain and its private key before they are installed
 *
 * Shared by the prompt and by the flags that answer it, so an answer given on
 * the command line is held to what the prompt would accept.
 *
 * @param {Object} paths
 * @param {string} paths.chainFilePath
 * @param {string} paths.privateFilePath
 * @return {boolean|string} true, or why the pair is refused
 */
export default function validateCertificateFilePaths({ chainFilePath, privateFilePath }) {
  if (!validateFileExists(chainFilePath)) {
    return 'certificate chain file path is not valid';
  }

  if (!validateFileExists(privateFilePath)) {
    return 'certificate key file path is not valid';
  }

  if (chainFilePath === privateFilePath) {
    return 'the same path for both files';
  }

  // Reading a FIFO or a device would wait or never end.
  const isRegularFile = (filePath) => fs.statSync(filePath).isFile();
  if (!isRegularFile(chainFilePath) || !isRegularFile(privateFilePath)) {
    return 'the certificate and key must be regular files';
  }

  let isValid;
  try {
    isValid = validateSslCertificateFiles(chainFilePath, privateFilePath);
  } catch {
    return 'The certificate or private key file cannot be read as PEM';
  }

  if (!isValid) {
    return 'The certificate and private key do not match';
  }

  return true;
}

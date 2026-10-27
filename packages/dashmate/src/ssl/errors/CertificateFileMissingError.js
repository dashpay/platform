import AbstractError from '../../errors/AbstractError.js';

/**
 * The certificate this node should be renewing is not on disk.
 *
 * Raised only where a read failed with `ENOENT`, and never from the error's
 * shape alone. The same read also fails for a permission denial and for a
 * corrupt file, and both of those are repaired locally - telling an operator to
 * obtain a certificate would spend one of a handful of weekly issuances on a
 * problem no certificate can fix. A provider response can also carry a `code`
 * property, so shape alone does not even establish that the failure was local.
 */
export default class CertificateFileMissingError extends AbstractError {
  /**
   * @param {string} certificatePath
   */
  constructor(certificatePath) {
    super(`This node's certificate file ${certificatePath} is missing`);

    this.certificatePath = certificatePath;
  }
}

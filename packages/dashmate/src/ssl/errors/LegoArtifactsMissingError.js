import AbstractError from '../../errors/AbstractError.js';

/**
 * The certificate helper succeeded, but the files it should have written are
 * not there.
 *
 * The most consequential of the failure outcomes to get right. A certificate
 * was issued, so it counts against this node's weekly issuance limit whether or
 * not dashmate can find it; retrying as though the authority had refused spends
 * that limit again for a problem that is entirely local.
 */
export default class LegoArtifactsMissingError extends AbstractError {
  /**
   * @param {string} missingPath
   */
  constructor(missingPath) {
    super(`The certificate was issued, but ${missingPath} was not written`);

    this.missingPath = missingPath;
  }
}

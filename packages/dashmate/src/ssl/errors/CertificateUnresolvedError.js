import AbstractError from '../../errors/AbstractError.js';

/**
 * The gateway certificate did not pass the checks and was not repaired.
 *
 * Thrown rather than recorded because a listr2 task can only render as failed
 * by throwing - there is no fail() on the task wrapper - and the operator has
 * to see which step failed. The list runs with exitOnError disabled so the
 * image pull still finishes and its table is still rendered; the command
 * recognises this error afterwards and separates it from a genuine fault.
 */
export default class CertificateUnresolvedError extends AbstractError {
  /**
   * @param {Object} verdict - as returned by checkGatewayCertificate
   */
  constructor(verdict) {
    super("The gateway certificate did not pass dashmate's checks");

    this.verdict = verdict;
  }

  /**
   * @return {Object}
   */
  getVerdict() {
    return this.verdict;
  }
}

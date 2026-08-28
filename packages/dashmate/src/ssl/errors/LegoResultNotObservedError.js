import AbstractError from '../../errors/AbstractError.js';

/**
 * The certificate helper ran, but dashmate never saw how it finished.
 *
 * Distinct from both of the other outcomes. A request may well have been made,
 * so it would be wrong to say nothing reached the authority - and no result was
 * read, so it is equally wrong to report what the authority said or to draw
 * conclusions about rate limits from a response nobody saw.
 */
export default class LegoResultNotObservedError extends AbstractError {
  /**
   * @param {Error} cause
   */
  constructor(cause) {
    super(`dashmate could not read the result of the certificate helper: ${cause.message}`);

    this.cause = cause;
  }
}

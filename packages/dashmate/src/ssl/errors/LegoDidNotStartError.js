import AbstractError from '../../errors/AbstractError.js';

/**
 * The certificate helper could not be started, so no request ever reached the
 * certificate authority.
 *
 * Worth distinguishing from a request the authority refused, because the two
 * have nothing in common: nothing was issued, nothing was validated, no rate
 * limit was spent and no address can have been paused. The most common cause is
 * another process already holding port 80, which is the opposite of the
 * firewall problem a failed validation usually means - the port is reachable,
 * it is occupied.
 */
export default class LegoDidNotStartError extends AbstractError {
  /**
   * @param {Error} cause - what Docker reported
   */
  constructor(cause) {
    super(`The certificate helper could not be started: ${cause.message}`);

    this.cause = cause;
  }
}

import AbstractError from '../../errors/AbstractError.js';

/**
 * The certificate helper could not be started.
 *
 * Worth distinguishing from a request the authority refused, because the two
 * have nothing in common. The most common cause is another process already
 * holding port 80, which is the opposite of the firewall problem a failed
 * validation usually means - the port is reachable, it is occupied.
 *
 * Whether the authority was reached is not always knowable. Failing to create
 * the container settles it: the helper never existed. Failing to start one that
 * was created does not, because Docker can reject after having accepted the
 * start, leaving the helper running and free to make its request.
 */
export default class LegoDidNotStartError extends AbstractError {
  /**
   * @param {Error} cause - what Docker reported
   * @param {boolean} [neverRan] - whether the helper is known not to have run
   */
  constructor(cause, neverRan = true) {
    super(`The certificate helper could not be started: ${cause.message}`);

    this.cause = cause;
    this.neverRan = neverRan;
  }
}

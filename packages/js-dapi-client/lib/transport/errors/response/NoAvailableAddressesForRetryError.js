import ResponseError from './ResponseError.js';

class NoAvailableAddressesForRetryError extends ResponseError {
  /**
   * @param {ResponseError} cause
   */
  constructor(cause) {
    super(
      cause.code,
      `No available addresses for retry: ${cause.message}`,
      cause.getData(),
      cause.getDAPIAddress(),
    );

    this.cause = cause;
  }

  /**
   * @returns {ResponseError}
   */
  getCause() {
    return this.cause;
  }
}

export default NoAvailableAddressesForRetryError;

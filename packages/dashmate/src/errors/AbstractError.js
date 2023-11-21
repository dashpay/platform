export default class AbstractError extends Error {
  /**
   * @param {string} message
   */
  constructor(message) {
    super();

    this.name = this.constructor.name;
    this.message = message;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

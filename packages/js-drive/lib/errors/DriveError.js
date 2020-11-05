class DriveError extends Error {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(message);

    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get message
   *
   * @return {string}
   */
  getMessage() {
    return this.message;
  }
}

module.exports = DriveError;

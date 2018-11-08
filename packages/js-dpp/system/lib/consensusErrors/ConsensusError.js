class ConsensusError extends Error {
  constructor(message) {
    super();

    this.name = this.constructor.name;
    this.deprecated = false;
    this.message = message;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Mark consensus rule as deprecated
   */
  markAsDeprecated() {
    this.deprecated = true;
  }

  isDeprecated() {
    return this.deprecated;
  }
}

module.exports = ConsensusError;

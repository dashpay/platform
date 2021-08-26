const ConsensusError = require('../../errors/consensus/ConsensusError');

class IncompatibleRe2PatternError extends ConsensusError {
  /**
   *
   * @param {string} pattern
   * @param {string} path
   * @param {string} originalErrorMessage
   */
  constructor(pattern, path, originalErrorMessage) {
    super(`Pattern ${pattern} at ${path} is not compatible with Re2: ${originalErrorMessage}`);

    this.pattern = pattern;
    this.path = path;
    this.originalErrorMessage = originalErrorMessage;
  }

  /**
   *
   * @returns {string}
   */
  getPattern() {
    return this.pattern;
  }

  /**
   *
   * @returns {string}
   */
  getPath() {
    return this.path;
  }

  /**
   *
   * @returns {string}
   */
  getOriginalErrorMessage() {
    return this.originalErrorMessage;
  }
}

module.exports = IncompatibleRe2PatternError;

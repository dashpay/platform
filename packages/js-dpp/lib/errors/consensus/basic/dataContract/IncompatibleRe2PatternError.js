const AbstractConsensusError = require('../../AbstractConsensusError');

class IncompatibleRe2PatternError extends AbstractConsensusError {
  /**
   *
   * @param {string} pattern
   * @param {string} path
   * @param {string} message
   */
  constructor(pattern, path, message) {
    super(`Pattern ${pattern} at ${path} is not compatible with Re2: ${message}`);

    this.pattern = pattern;
    this.path = path;
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
   * @param {Error} error
   */
  setPatternError(error) {
    this.patternError = error;
  }

  /**
   * @returns {Error}
   */
  getPatternError() {
    return this.patternError;
  }
}

module.exports = IncompatibleRe2PatternError;

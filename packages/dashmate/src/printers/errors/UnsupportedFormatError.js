const AbstractError = require('../../errors/AbstractError');

class UnsupportedFormatError extends AbstractError {
  /**
   * @param {string} formatName
   */
  constructor(formatName) {
    super(`Unsupported format: ${formatName}`);

    this.formatName = formatName;
  }

  /**
   * Get config name
   *
   * @return {string}
   */
  getformatName() {
    return this.formatName;
  }
}

module.exports = UnsupportedFormatError;

const DriveError = require('../../../errors/DriveError');

class InvalidMasternodeIdentityError extends DriveError {
  /**
   * @param {Error} validationError
   */
  constructor(validationError) {
    super('Invalid masternode identity');

    this.validationError = validationError;
  }

  /**
   * Get validation error
   *
   * @return {Error}
   */
  getValidationError() {
    return this.validationError;
  }
}

module.exports = InvalidMasternodeIdentityError;

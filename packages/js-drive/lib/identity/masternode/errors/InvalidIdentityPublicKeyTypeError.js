const DriveError = require('../../../errors/DriveError');

class InvalidIdentityPublicKeyTypeError extends DriveError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super('Invalid Identity Public Key type');

    this.type = type;
  }

  /**
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = InvalidIdentityPublicKeyTypeError;

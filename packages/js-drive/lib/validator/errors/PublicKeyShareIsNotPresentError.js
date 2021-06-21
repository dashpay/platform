const DriveError = require('../../errors/DriveError');

class PublicKeyShareIsNotPresentError extends DriveError {
  /**
   * @param {Object} member
   */
  constructor(member) {
    super('Public key share is not present for validator');

    this.member = member;
  }

  /**
   * Get quorum member info
   *
   * @return {Object}
   */
  getMember() {
    return this.member;
  }
}

module.exports = PublicKeyShareIsNotPresentError;

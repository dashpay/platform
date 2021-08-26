const AbstractBasicError = require('../AbstractBasicError');

class InvalidDataContractIdError extends AbstractBasicError {
  /**
   * @param {Buffer} expectedId
   * @param {Buffer} invalidId
   */
  constructor(expectedId, invalidId) {
    super(`DataContract ID must be ${expectedId.toString('hex')}, got ${invalidId.toString('hex')}`);

    this.expectedId = expectedId;
    this.invalidId = invalidId;
  }

  /**
   * @return {Buffer}
   */
  getExpectedId() {
    return this.expectedId;
  }

  /**
   * @return {Buffer}
   */
  getInvalidId() {
    return this.invalidId;
  }
}

module.exports = InvalidDataContractIdError;

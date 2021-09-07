const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

class InvalidDocumentTransitionIdError extends AbstractBasicError {
  /**
   * @param {Buffer} expectedId
   * @param {Buffer} invalidId
   */
  constructor(expectedId, invalidId) {
    super(`Invalid document transition id ${Identifier.from(invalidId)}, expected ${Identifier.from(expectedId)}`);

    this.expectedId = expectedId;
    this.invalidId = invalidId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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

module.exports = InvalidDocumentTransitionIdError;

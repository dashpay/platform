const AbstractBasicError = require('../AbstractBasicError');

class InvalidDataContractVersionError extends AbstractBasicError {
  /**
   * @param {number} expectedVersion
   * @param {number} version
   */
  constructor(expectedVersion, version) {
    super(`Data Contract version must be ${expectedVersion}, got ${version}`);

    this.expectedVersion = expectedVersion;
    this.version = version;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {number}
   */
  getExpectedVersion() {
    return this.expectedVersion;
  }

  /**
   * @return {number}
   */
  getVersion() {
    return this.version;
  }
}

module.exports = InvalidDataContractVersionError;

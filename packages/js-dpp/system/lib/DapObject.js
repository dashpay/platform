const InvalidDapObjectTypeError = require('./errors/InvalidDapObjectTypeError');

/**
 * @class DapObject
 * @property $$type
 */
class DapObject {
  /**
   * @param {string} type
   * @param {DapContract} dapContract
   */
  constructor(type, dapContract) {
    if (!dapContract.isDapObjectTypeDefined(type)) {
      throw new InvalidDapObjectTypeError(type, dapContract);
    }
    this.$$type = type;
  }
}

module.exports = DapObject;

const InvalidObjectTypeError = require('./errors/InvalidObjectTypeError');

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
    if (!dapContract.isObjectTypeDefined(type)) {
      throw new InvalidObjectTypeError(type, dapContract);
    }
    this.$$type = type;
  }
}

module.exports = DapObject;

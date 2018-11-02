const { InvalidObjectType } = require('./errors');

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
      throw new InvalidObjectType();
    }
    this.$$type = type;
  }
}

module.exports = DapObject;

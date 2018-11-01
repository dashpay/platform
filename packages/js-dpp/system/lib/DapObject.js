const { InvalidObjectType } = require('./errors');

/**
 * @class DapObject
 * @property _type
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
    this._type = type;
  }
}

module.exports = DapObject;

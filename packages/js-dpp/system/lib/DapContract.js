/**
 * @class DapContract
 * @property {string} name
 * @property {number} version
 * @property {Object} objectsDefinition
 */
class DapContract {
  /**
   * Returns true if object type is defined in this dap contract
   * @param {string} type
   * @return {boolean}
   */
  isObjectTypeDefined(type) {
    return Object.prototype.hasOwnProperty.call(this.objectsDefinition, type);
  }
}

module.exports = DapContract;

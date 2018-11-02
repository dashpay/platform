/**
 * @class DapContract
 * @property {string} name
 * @property {number} version
 * @property {Object} dapObjectsDefinition
 */
class DapContract {
  /**
   * Returns true if object type is defined in this dap contract
   * @param {string} type
   * @return {boolean}
   */
  isDapObjectTypeDefined(type) {
    return Object.prototype.hasOwnProperty.call(this.dapObjectsDefinition, type);
  }
}

module.exports = DapContract;

/**
 * @abstract
 */
class AbstractOperation {
  /**
   * @abstract
   * @returns {number}
   */
  getProcessingCost() {
    throw new Error('Not implemented');
  }

  /**
   * @abstract
   * @returns {number}
   */
  getStorageCost() {
    throw new Error('Not implemented');
  }
}

module.exports = AbstractOperation;

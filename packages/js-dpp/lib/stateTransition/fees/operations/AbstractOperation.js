/**
 * @abstract
 */
class AbstractOperation {
  /**
   * @abstract
   * @returns {number}
   */
  getCpuCost() {
    throw new Error('Not implemented');
  }

  /**
   * @abstract
   * @returns {number}
   */
  getStorageCost() {
    throw new Error('Not implemented');
  }

  /**
   * @abstract
   * @return {string}
   */
  getType() {
    throw new Error('Not implemented');
  }
}

AbstractOperation.STORAGE_CREDIT_PER_BYTE = 5000;
AbstractOperation.STORAGE_PROCESSING_CREDIT_PER_BYTE = 10;

AbstractOperation.TYPES = {
  WRITE: 'write',
  READ: 'read',
  DELETE: 'delete',
  PRE_CALCULATED: 'preCalculated',
};

module.exports = AbstractOperation;

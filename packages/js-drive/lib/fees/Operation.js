class Operation {
  /**
   * @returns {number}
   */
  getCpuCost() {
    throw new Error('Not implemented');
  }

  /**
   * @returns {number}
   */
  getStorageCost() {
    throw new Error('Not implemented');
  }

  /**
   * @return {string}
   */
  getType() {
    throw new Error('Not implemented');
  }
}

Operation.STORAGE_CREDIT_PER_BYTE = 5000;
Operation.STORAGE_PROCESSING_CREDIT_PER_BYTE = 10;
Operation.QUERY_CREDIT_PER_BYTE = 10;

Operation.TYPES = {
  WRITE: 'write',
  READ: 'read',
  DELETE: 'delete',
  PRE_CALCULATED: 'preCalculated',
};

module.exports = Operation;
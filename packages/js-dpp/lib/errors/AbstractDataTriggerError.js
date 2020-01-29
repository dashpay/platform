const ConsensusError = require('./ConsensusError');

/**
 * @abstract
 */
class AbstractDataTriggerError extends ConsensusError {
  /**
   * @param {string} message
   * @param {DataContract} dataContract
   * @param {string} userId
   */
  constructor(message, dataContract, userId) {
    super(message);

    this.dataContract = dataContract;
    this.userId = userId;
  }

  /**
   * Get data trigger data contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * Get data trigger user id
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }
}

module.exports = AbstractDataTriggerError;

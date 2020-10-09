const ConsensusError = require('./ConsensusError');

/**
 * @abstract
 */
class AbstractDataTriggerError extends ConsensusError {
  /**
   * @param {string} message
   * @param {DataContract} dataContract
   * @param {Identifier} ownerId
   */
  constructor(message, dataContract, ownerId) {
    super(message);

    this.dataContract = dataContract;
    this.ownerId = ownerId;
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
   * Get data trigger owner id
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.ownerId;
  }
}

module.exports = AbstractDataTriggerError;

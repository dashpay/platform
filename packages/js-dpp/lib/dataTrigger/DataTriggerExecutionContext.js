const Identifier = require('../Identifier');

class DataTriggerExecutionContext {
  /**
   * @param {StateRepository} stateRepository
   * @param {Buffer} ownerId
   * @param {DataContract} dataContract
   */
  constructor(stateRepository, ownerId, dataContract) {
    /**
     * @type {StateRepository}
     */
    this.stateRepository = stateRepository;
    this.ownerId = Identifier.from(ownerId);
    this.dataContract = dataContract;
  }

  /**
   * @returns {StateRepository}
   */
  getStateRepository() {
    return this.stateRepository;
  }

  /**
   * @returns {Identifier}
   */
  getOwnerId() {
    return this.ownerId;
  }

  /**
   * @returns {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = DataTriggerExecutionContext;

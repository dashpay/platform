class DataTriggerExecutionContext {
  /**
   * @param {StateRepository} stateRepository
   * @param {string} ownerId
   * @param {DataContract} dataContract
   */
  constructor(stateRepository, ownerId, dataContract) {
    /**
     * @type {StateRepository}
     */
    this.stateRepository = stateRepository;
    this.ownerId = ownerId;
    this.dataContract = dataContract;
  }

  /**
   * @returns {StateRepository}
   */
  getStateRepository() {
    return this.stateRepository;
  }

  /**
   * @returns {string}
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

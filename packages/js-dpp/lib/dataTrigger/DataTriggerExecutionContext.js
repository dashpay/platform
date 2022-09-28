class DataTriggerExecutionContext {
  /**
   * @param {StateRepository} stateRepository
   * @param {Buffer|Identifier} ownerId
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} stateTransitionExecutionContext
   */
  constructor(stateRepository, ownerId, dataContract, stateTransitionExecutionContext) {
    /**
     * @type {StateRepository}
     */
    this.stateRepository = stateRepository;
    this.ownerId = ownerId;
    this.dataContract = dataContract;
    this.stateTransitionExecutionContext = stateTransitionExecutionContext;
  }

  /**
   * @returns {StateRepository}
   */
  getStateRepository() {
    return this.stateRepository;
  }

  /**
   * @returns {Buffer|Identifier}
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

  /**
   * @return {StateTransitionExecutionContext}
   */
  getStateTransitionExecutionContext() {
    return this.stateTransitionExecutionContext;
  }
}

module.exports = DataTriggerExecutionContext;

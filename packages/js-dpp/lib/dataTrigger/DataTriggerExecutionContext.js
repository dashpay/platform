class DataTriggerExecutionContext {
  /**
   * @param {DataProvider} dataProvider
   * @param {string} userId
   * @param {DataContract} dataContract
   * @param {Transaction} stateTransitionHeader
   */
  constructor(dataProvider, userId, dataContract, stateTransitionHeader) {
    /**
     * @type {DataProvider}
     */
    this.dataProvider = dataProvider;
    this.userId = userId;
    this.dataContract = dataContract;
    this.stateTransitionHeader = stateTransitionHeader;
  }

  /**
   * @returns {DataProvider}
   */
  getDataProvider() {
    return this.dataProvider;
  }

  /**
   * @returns {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * @returns {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * @returns {Transaction}
   */
  getStateTransitionHeader() {
    return this.stateTransitionHeader;
  }
}

module.exports = DataTriggerExecutionContext;

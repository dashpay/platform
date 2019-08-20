class DataTriggerExecutionContext {
  /**
   * @param {DataProvider} dataProvider
   * @param {string} userId
   * @param {Contract} contract
   * @param {Transaction} stateTransitionHeader
   */
  constructor(dataProvider, userId, contract, stateTransitionHeader) {
    /**
     * @type {DataProvider}
     */
    this.dataProvider = dataProvider;
    this.userId = userId;
    this.contract = contract;
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
   * @returns {Contract}
   */
  getContract() {
    return this.contract;
  }

  /**
   * @returns {Transaction}
   */
  getStateTransitionHeader() {
    return this.stateTransitionHeader;
  }
}

module.exports = DataTriggerExecutionContext;

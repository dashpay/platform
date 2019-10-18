class DataTriggerExecutionContext {
  /**
   * @param {DataProvider} dataProvider
   * @param {string} userId
   * @param {DataContract} dataContract
   */
  constructor(dataProvider, userId, dataContract) {
    /**
     * @type {DataProvider}
     */
    this.dataProvider = dataProvider;
    this.userId = userId;
    this.dataContract = dataContract;
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
}

module.exports = DataTriggerExecutionContext;

class DataContractHistoryEntry {
  /**
   * @param {bigint} date - timestamp
   * @param {Buffer} value - buffer value of the data contract
   */
  constructor(date, value) {
    this.date = date;
    this.value = value;
  }

  /**
   * @returns {bigint} - date
   */
  getDate() {
    return this.date;
  }

  /**
   * @returns {Buffer} - raw binary value of the data contract
   */
  getValue() {
    return this.value;
  }
}

module.exports = DataContractHistoryEntry;

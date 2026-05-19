class DataContractHistoryEntry {
  /**
   * @param {bigint} date - timestamp
   * @param {Uint8Array} value - byte value of the data contract
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
   * @returns {Uint8Array} - raw binary value of the data contract
   */
  getValue() {
    return this.value;
  }
}

export default DataContractHistoryEntry;

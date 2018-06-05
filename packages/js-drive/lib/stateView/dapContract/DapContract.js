class DapContract {
  /**
   * @param {string} dapId
   * @param {string} dapName
   * @param {string} packetHash
   * @param {object} schema
   */
  constructor(dapId, dapName, packetHash, schema) {
    this.dapId = dapId;
    this.dapName = dapName;
    this.packetHash = packetHash;
    this.schema = schema;
  }

  getDapId() {
    return this.dapId;
  }

  getDapName() {
    return this.dapName;
  }

  getSchema() {
    return this.schema;
  }

  /**
   * Get DapContract JSON representation
   *
   * @returns {{dapId: string, dapName: string, packetHash: string, schema: Object}}
   */
  toJSON() {
    return {
      dapId: this.dapId,
      dapName: this.dapName,
      packetHash: this.packetHash,
      schema: this.schema,
    };
  }
}

module.exports = DapContract;

class DapContract {
  /**
   * @param {string} dapId
   * @param {string} dapName
   * @param {Reference} reference
   * @param {object} schema
   * @param {number} version
   * @param {array} previousVersions
   */
  constructor(dapId, dapName, reference, schema, version, previousVersions = []) {
    this.dapId = dapId;
    this.dapName = dapName;
    this.reference = reference;
    this.schema = schema;
    this.version = version;
    this.previousVersions = previousVersions;
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

  getVersion() {
    return this.version;
  }

  getPreviousVersions() {
    return this.previousVersions;
  }

  currentRevision() {
    return {
      version: this.version,
      reference: this.reference,
    };
  }

  addRevision(previousDapContract) {
    this.previousVersions = this.previousVersions
      .concat(previousDapContract.getPreviousVersions())
      .concat([previousDapContract.currentRevision()]);
  }

  /**
   * Get DapContract JSON representation
   *
   * @returns {{dapId: string, dapName: string, reference: Object,
   *              schema: Object, version: number, previousVersions: array}}
   */
  toJSON() {
    return {
      dapId: this.dapId,
      dapName: this.dapName,
      reference: this.reference.toJSON(),
      schema: this.schema,
      version: this.version,
      previousVersions: this.previousVersionsToJSON(),
    };
  }

  /**
   * @private
   * @returns {{version: number,
   *            reference: {blockHash, blockHeight, stHeaderHash, stPacketHash, objectHash}}[]}
   */
  previousVersionsToJSON() {
    return this.previousVersions.map(previousRevision => ({
      version: previousRevision.version,
      reference: previousRevision.reference.toJSON(),
    }));
  }
}

module.exports = DapContract;

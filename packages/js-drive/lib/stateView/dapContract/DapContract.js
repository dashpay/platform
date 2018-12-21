class DapContract {
  /**
   * @param {string} dapId
   * @param {object} data
   * @param {Reference} reference
   * @param {boolean} isDeleted
   * @param {array} previousVersions
   */
  constructor(dapId, data, reference, isDeleted, previousVersions = []) {
    const instance = this;
    ({
      dapname: instance.dapName,
      dapver: instance.version,
      ...instance.data
    } = data);

    this.dapId = dapId;
    this.reference = reference;
    this.deleted = isDeleted;
    this.previousVersions = previousVersions;
  }

  getDapId() {
    return this.dapId;
  }

  getDapName() {
    return this.dapName;
  }

  getOriginalData() {
    return {
      dapname: this.dapName,
      dapver: this.version,
      ...this.data,
    };
  }

  getVersion() {
    return this.version;
  }

  isDeleted() {
    return this.deleted;
  }

  markAsDeleted() {
    this.deleted = true;
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

  removeAheadRevisions() {
    this.previousVersions = this.previousVersions
      .filter(({ version }) => version < this.getVersion());
  }

  /**
   * Get DapContract JSON representation
   *
   * @returns {{dapId: string, dapName: string, reference: Object,
   *              data: Object, version: number,
   *              isDeleted: boolean,
   *              previousVersions: array}}
   */
  toJSON() {
    return {
      dapId: this.dapId,
      dapName: this.dapName,
      reference: this.reference.toJSON(),
      data: this.data,
      version: this.version,
      isDeleted: this.deleted,
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

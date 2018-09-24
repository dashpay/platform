class SyncState {
  /**
   * @param {Object[]} blocks
   * @param {Date} lastSyncAt
   */
  constructor(blocks, lastSyncAt) {
    this.setBlocks(blocks);
    this.setLastSyncAt(lastSyncAt);
  }

  /**
   * Set blocks
   *
   * @param {Object[]} blocks
   */
  setBlocks(blocks) {
    this.blocks = blocks;
  }

  /**
   * Get blocks
   *
   * @return {Object[]}
   */
  getBlocks() {
    return this.blocks;
  }

  /**
   * Set lastInitialSyncAt
   *
   * @param {Date} date
   */
  setLastInitialSyncAt(date) {
    this.lastInitialSyncAt = date;
  }

  /**
   * Set lastInitialSyncAt
   *
   * @return {Date}
   */
  getLastInitialSyncAt() {
    return this.lastInitialSyncAt;
  }

  /**
   * Set last sync date
   *
   * @param {Date} date
   * @return {number}
   */
  setLastSyncAt(date) {
    if (!this.lastSyncAt) {
      this.setLastInitialSyncAt(date);
    }

    this.lastSyncAt = date;
  }

  /**
   * Get last sync date
   *
   * @return {Date}
   */
  getLastSyncAt() {
    return this.lastSyncAt;
  }

  /**
   * Get last block
   *
   * @return {Object}
   */
  getLastBlock() {
    return this.blocks[this.blocks.length - 1] || {};
  }

  /**
   * Get last block hash
   *
   * @return {string}
   */
  getLastBlockHash() {
    return this.blocks[this.blocks.length - 1].hash;
  }

  /**
   * Compare with another state instance
   *
   * @param {SyncState} state
   * @return {boolean}
   */
  isEqual(state) {
    return JSON.stringify(this.toJSON()) === JSON.stringify(state.toJSON());
  }

  isEmpty() {
    return !this.blocks.length && !this.lastSyncAt;
  }

  /**
   * Clean SyncState
   */
  clean() {
    this.setBlocks([]);
    this.setLastSyncAt(null);
    this.setLastInitialSyncAt(null);
  }

  /**
   * Get state's JSON representation
   *
   * @return {{blocks: Object[], lastSyncAt: Date, lastInitialSyncAt: Date}}
   */
  toJSON() {
    return {
      blocks: this.getBlocks(),
      lastSyncAt: this.getLastSyncAt(),
      lastInitialSyncAt: this.getLastInitialSyncAt(),
    };
  }
}

module.exports = SyncState;

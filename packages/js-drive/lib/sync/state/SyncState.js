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
   * Set last sync date
   *
   * @param {Date} date
   * @return {number}
   */
  setLastSyncAt(date) {
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
   * @return {Object|null}
   */
  getLastBlock() {
    return this.blocks[this.blocks.length - 1];
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

  /**
   * Get state's JSON representation
   *
   * @return {{blocks: Object[], lastSyncAt: Date}}
   */
  toJSON() {
    return {
      blocks: this.getBlocks(),
      lastSyncAt: this.getLastSyncAt(),
    };
  }
}

module.exports = SyncState;

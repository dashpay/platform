class SyncInfo {
  /**
   * @param {string} lastSyncedBlockHeight
   * @param {string} lastSyncedBlockHash
   * @param {Date} lastSyncAt
   * @param {string} lastChainBlockHeight
   * @param {string} lastChainBlockHash
   * @param {boolean} isBlockchainSynced
   */
  constructor(
    lastSyncedBlockHeight,
    lastSyncedBlockHash,
    lastSyncAt,
    lastChainBlockHeight,
    lastChainBlockHash,
    isBlockchainSynced,
  ) {
    this.lastSyncedBlockHeight = lastSyncedBlockHeight;
    this.lastSyncedBlockHash = lastSyncedBlockHash;
    this.lastSyncAt = lastSyncAt;
    this.lastChainBlockHeight = lastChainBlockHeight;
    this.lastChainBlockHash = lastChainBlockHash;
    this.isBlockchainSynced = isBlockchainSynced;
  }

  /**
   * Get Last synced block height
   *
   * @returns {string}
   */
  getLastSyncedBlockHeight() {
    return this.lastSyncedBlockHeight;
  }

  /**
   * Get Last synced block hash
   *
   * @returns {string}
   */
  getLastSyncedBlockHash() {
    return this.lastSyncedBlockHash;
  }

  /**
   * Get Last sync date
   *
   * @returns {Date}
   */
  getLastSyncAt() {
    return this.lastSyncAt;
  }

  /**
   * Get Last blockchain height
   *
   * @returns {string}
   */
  getLastChainBlockHeight() {
    return this.lastChainBlockHeight;
  }

  /**
   * Get Last blockchain hash
   *
   * @returns {string}
   */
  getLastChainBlockHash() {
    return this.lastChainBlockHash;
  }

  /**
   * Get Drive sync status
   *
   * @returns {string}
   */
  getStatus() {
    if (!this.lastSyncAt) {
      return SyncInfo.STATUSES.INITIAL_SYNC;
    }

    if (!this.isBlockchainSynced) {
      return SyncInfo.STATUSES.SYNCING;
    }

    if (this.lastSyncedBlockHash !== this.lastChainBlockHash) {
      return SyncInfo.STATUSES.SYNCING;
    }

    return SyncInfo.STATUSES.SYNCED;
  }

  /**
   * Returns SyncInfo JSON representation
   *
   * @returns {{lastSyncedBlockHeight: string, lastSyncedBlockHash: string, lastSyncAt: Date,
   *                lastChainBlockHeight: string, lastChainBlockHash: string, status: string}}
   */
  toJSON() {
    return {
      lastSyncedBlockHeight: this.lastSyncedBlockHeight,
      lastSyncedBlockHash: this.lastSyncedBlockHash,
      lastSyncAt: this.lastSyncAt,
      lastChainBlockHeight: this.lastChainBlockHeight,
      lastChainBlockHash: this.lastChainBlockHash,
      status: this.getStatus(),
    };
  }
}

SyncInfo.STATUSES = {
  INITIAL_SYNC: 'initialSync',
  SYNCING: 'syncing',
  SYNCED: 'synced',
};

module.exports = SyncInfo;

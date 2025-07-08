class StateSyncStatus {
  /**
   * @param {bigint} totalSyncedTime - Total synced time
   * @param {bigint} remainingTime - Remaining time to sync
   * @param {number} totalSnapshots - Total snapshots count
   * @param {bigint} chunkProcessAverageTime - Chunk process average time
   * @param {bigint} snapshotHeight - Snapshot height
   * @param {bigint} snapshotChunksCount - Snapshot chunks count
   * @param {bigint} backfilledBlocks - Backfilled blocks
   * @param {bigint} backfillBlocksTotal - Backfilled blocks total count
   */
  constructor(
    totalSyncedTime,
    remainingTime,
    totalSnapshots,
    chunkProcessAverageTime,
    snapshotHeight,
    snapshotChunksCount,
    backfilledBlocks,
    backfillBlocksTotal,
  ) {
    this.totalSyncedTime = totalSyncedTime;
    this.remainingTime = remainingTime;
    this.totalSnapshots = totalSnapshots;
    this.chunkProcessAverageTime = chunkProcessAverageTime;
    this.snapshotHeight = snapshotHeight;
    this.snapshotChunksCount = snapshotChunksCount;
    this.backfilledBlocks = backfilledBlocks;
    this.backfillBlocksTotal = backfillBlocksTotal;
  }

  /**
   * @returns {bigint} Total synced time
   */
  getTotalSyncedTime() {
    return this.totalSyncedTime;
  }

  /**
   * @returns {bigint} Total synced time
   */
  getRemainingTime() {
    return this.remainingTime;
  }

  /**
   * @returns {number} Total snapshots count
   */
  getTotalSnapshots() {
    return this.totalSnapshots;
  }

  /**
   * @returns {bigint} Chunk process average time
   */
  getChunkProcessAverageTime() {
    return this.chunkProcessAverageTime;
  }

  /**
   * @returns {bigint} Chunk process average time
   */
  getSnapshotHeight() {
    return this.snapshotHeight;
  }

  /**
   * @returns {bigint} Chunk process average time
   */
  getSnapshotChunkCount() {
    return this.snapshotChunksCount;
  }

  /**
   * @returns {bigint} Backfilled blocks
   */
  getBackfilledBlocks() {
    return this.backfilledBlocks;
  }

  /**
   * @returns {bigint} Backfill blocks total
   */
  getBackfilledBlockTotal() {
    return this.backfillBlocksTotal;
  }
}

module.exports = StateSyncStatus;

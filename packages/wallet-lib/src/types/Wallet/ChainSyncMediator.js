/* eslint-disable no-underscore-dangle */
const EVENTS = require('../../EVENTS');

const STATES = {
  OFFLINE: 'OFFLINE',
  CHAIN_STATUS_SYNC: 'CHAIN_STATUS_SYNC',
  HISTORICAL_SYNC: 'HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'CONTINUOUS_SYNC',
};

/**
 * The class responsible for communication of
 * the chain sync state between plugins
 * @class ChainSyncMediator
 */
class ChainSyncMediator {
  constructor() {
    this._state = STATES.OFFLINE;
    this.blockHeights = {};
    this.transactionsBlockHashes = {};
    this.txChunkHashes = new Set();
    /**
     * The last hash synced by transactions and proofs stream
     * @type {string}
     */
    this.lastSyncedMerkleBlockHash = '';
    this.lastSyncedHeaderHeight = 1;
    this.lastHeadersCount = 1;
    this.totalHeadersCount = -1;
  }

  /**
   * Function checks if any update has happened
   * @param {EventEmitter} [eventBus]
   * @returns {number}
   */
  updateProgress(eventBus) {
    const totalSyncedHeaders = Object.keys(this.blockHeights).length;

    if (totalSyncedHeaders < this.lastHeadersCount) {
      // TODO: test - remove
      throw new Error('You are calculated block count not right');
    }

    if (totalSyncedHeaders === this.lastHeadersCount) {
      return this.lastSyncedHeaderHeight;
    }

    const confirmedSyncedHeaders = this.blockHeights[this.lastSyncedMerkleBlockHash] || 0;
    let unconfirmedSyncedHeaders = totalSyncedHeaders - confirmedSyncedHeaders - 1;
    unconfirmedSyncedHeaders = unconfirmedSyncedHeaders > 0 ? unconfirmedSyncedHeaders : 0;
    if (confirmedSyncedHeaders) {
      this.lastSyncedHeaderHeight = confirmedSyncedHeaders;
    }

    const confirmedProgress = Math.round(
      (confirmedSyncedHeaders / this.totalHeadersCount) * 10000,
    ) / 10000;
    const unconfirmedProgress = Math.round(
      (unconfirmedSyncedHeaders / this.totalHeadersCount) * 10000,
    ) / 10000;
    const totalProgress = Math.round(
      (totalSyncedHeaders / this.totalHeadersCount) * 10000,
    ) / 10000;

    // console.log('[ChainSyncMediator]',
    // this.lastSyncedMerkleBlockHash, Object.keys(this.blockHeights).length);
    // if (lastSyncedBlockHeight > 0) {
    console.log('Last known merkle block', this.lastSyncedMerkleBlockHash, this.lastSyncedHeaderHeight);
    // }
    // console.log(`Progress. Confirmed: ${confirmedProgress},
    // unconfirmed: ${unconfirmedProgress}, total: ${totalProgress},
    // ${confirmedProgress + unconfirmedProgress}`);
    eventBus.emit(EVENTS.SYNC_PROGRESS, totalProgress);
    eventBus.emit(EVENTS.SYNC_PROGRESS_CONFIRMED, confirmedProgress);
    eventBus.emit(EVENTS.SYNC_PROGRESS_UNCONFIRMED, unconfirmedProgress);

    this.lastHeadersCount = totalSyncedHeaders;

    return this.lastSyncedHeaderHeight;
  }

  /**
   * Changes the state of the chain sync
   * @param {string} state
   */
  set state(state) {
    if (!STATES[state]) {
      throw new Error('Invalid state');
    }

    this._state = state;
  }

  /**
   * Returns the current state of the chain sync
   * @returns {string}
   */
  get state() {
    return this._state;
  }
}

ChainSyncMediator.STATES = STATES;

module.exports = ChainSyncMediator;

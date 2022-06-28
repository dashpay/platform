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
     * Pending block hashes to be verified
     * @type {*[]}
     */
    this.blockHashesToVerify = [];

    this.lastSyncedHeaderHeight = 1;
    this.lastHeadersCount = 1;
    this.totalHeadersCount = -1;
    this.progressUpdatePromise = null;
  }

  scheduleProgressUpdate(eventBus) {
    if (!this.progressUpdatePromise) {
      this.progressUpdatePromise = new Promise((resolve) => {
        setTimeout(() => {
          const lastSyncedBlockHeight = this.updateProgress(eventBus);
          this.progressUpdatePromise = null;
          resolve(lastSyncedBlockHeight);
        }, 1000);
      });

      // this.progressUpdateTimeout = setTimeout(() => {
      //   this.updateProgress(eventBus);
      //   this.progressUpdateTimeout = null;
      // }, 1000);
    }

    return this.progressUpdatePromise;
  }

  /**
   * Function checks if any update has happened
   * @param {EventEmitter} [eventBus]
   * @returns {number}
   */
  updateProgress(eventBus) {
    const totalSyncedHeaders = Object.keys(this.blockHeights).length;

    // if (totalSyncedHeaders === this.lastHeadersCount) {
    //   return this.lastSyncedHeaderHeight;
    // }

    let confirmedSyncedHeaders = this.lastSyncedHeaderHeight;

    for (let i = this.blockHashesToVerify.length - 1; i >= 0; i -= 1) {
      const hash = this.blockHashesToVerify[i];
      const height = this.blockHeights[hash];
      if (typeof height === 'number') {
        confirmedSyncedHeaders = height;
        this.blockHashesToVerify.splice(0, i + 1);
        break;
      }
    }

    let unconfirmedSyncedHeaders = totalSyncedHeaders - confirmedSyncedHeaders - 1;
    unconfirmedSyncedHeaders = unconfirmedSyncedHeaders > 0 ? unconfirmedSyncedHeaders : 0;

    if (confirmedSyncedHeaders > this.lastSyncedHeaderHeight) {
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

    console.log(
      'Verified height', this.lastSyncedHeaderHeight,
      'last known block', this.blockHashesToVerify[this.blockHashesToVerify.length - 1],
    );

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

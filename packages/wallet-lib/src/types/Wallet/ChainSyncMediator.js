/* eslint-disable no-underscore-dangle */

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

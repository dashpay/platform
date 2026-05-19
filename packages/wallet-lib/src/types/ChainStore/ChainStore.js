import EventEmitter from 'events';

import dashcore from '@dashevo/dashcore-lib';
const {
  Transaction,
  BlockHeader,
} = dashcore;
import CONSTANTS from '../../CONSTANTS.js';

const SCHEMA = {
  lastSyncedHeaderHeight: 'number',
  lastSyncedBlockHeight: 'number',
  chainHeight: 'number',
  blockHeaders: [
    (hex) => new BlockHeader(Buffer.from(hex, 'hex')),
  ],
  transactions: {
    '*': Transaction,
  },
  txMetadata: {
    '*': {
      blockHash: 'string',
      height: 'number',
      time: 'number',
      isChainLocked: 'boolean',
      isInstantLocked: 'boolean',
    },
  },
  fees: {
    minRelay: 'number',
  },
};

/**
 * ChainStore holds any information that is relatives to a specific network.
 * Information such as blockHeaders, transactions, instantLocks.
 * Also holds the state of addresses based on the transactions imported (e.g: balances and utxos).
 */
class ChainStore extends EventEmitter {
  constructor(networkIdentifier = 'testnet') {
    super();
    this.network = networkIdentifier;
    this.headersToKeep = CONSTANTS.STORAGE.headersToKeep;

    this.reset();
  }

  reset() {
    this.state = {
      fees: {
        minRelay: -1,
      },
      // Height of the last synced header
      lastSyncedHeaderHeight: -1,
      // Height of the last synced merkle block
      lastSyncedBlockHeight: -1,
      chainHeight: 0,
      blockHeaders: [],
      headersMetadata: new Map(),
      hashesByHeight: new Map(),
      transactions: new Map(),
      instantLocks: new Map(),
      addresses: new Map(),
    };
  }

  getTransactions() {
    return this.state.transactions;
  }

  setBlockHeaders(headers) {
    this.state.blockHeaders = headers;
  }

  /**
   * Sets headers metadata and hashes by height
   * @param headers
   * @param tipHeight
   */
  updateHeadersMetadata(headers, tipHeight) {
    headers.forEach((header, index) => {
      const height = tipHeight - headers.length + index + 1;
      this.state.headersMetadata.set(header.hash, {
        height,
        time: header.time,
      });

      this.state.hashesByHeight.set(height, header.hash);
    });
  }

  /**
   * Prunes headers metadata starting lower than specified height
   * @param {number} belowHeight
   */
  pruneHeadersMetadata(belowHeight) {
    let currentHeight = belowHeight - 1;
    let currentHash = this.state.hashesByHeight.get(currentHeight);

    while (currentHash) {
      this.state.headersMetadata.delete(currentHash);
      this.state.hashesByHeight.delete(currentHeight);
      currentHeight -= 1;
      currentHash = this.state.hashesByHeight.get(currentHeight);
    }
  }

  updateLastSyncedHeaderHeight(height) {
    if (height < this.state.lastSyncedHeaderHeight) {
      throw new Error(`Cannot update lastSyncedHeaderHeight to a lower value ${height} < ${this.state.lastSyncedHeaderHeight}`);
    }

    this.state.lastSyncedHeaderHeight = height;
  }

  updateLastSyncedBlockHeight(height) {
    if (height < this.state.lastSyncedBlockHeight) {
      throw new Error(`Cannot update lastSyncedBlockHeight to a lower value ${height} < ${this.state.lastSyncedBlockHeight}`);
    }

    this.state.lastSyncedBlockHeight = height;
  }

  updateChainHeight(height) {
    if (height < this.state.chainHeight) {
      throw new Error(`Chain height value ${height} is lower than current value ${this.state.chainHeight}`);
    }

    this.state.chainHeight = height;
  }
}

ChainStore.prototype.SCHEMA = SCHEMA;

import _ChainStore_considerTransaction from './methods/considerTransaction.js';
ChainStore.prototype.considerTransaction = _ChainStore_considerTransaction;

import _ChainStore_exportState from './methods/exportState.js';
ChainStore.prototype.exportState = _ChainStore_exportState;
import _ChainStore_importState from './methods/importState.js';
ChainStore.prototype.importState = _ChainStore_importState;

import _ChainStore_getAddress from './methods/getAddress.js';
ChainStore.prototype.getAddress = _ChainStore_getAddress;
import _ChainStore_getAddresses from './methods/getAddresses.js';
ChainStore.prototype.getAddresses = _ChainStore_getAddresses;

import _ChainStore_getBlockHeader from './methods/getBlockHeader.js';
ChainStore.prototype.getBlockHeader = _ChainStore_getBlockHeader;
import _ChainStore_getInstantLock from './methods/getInstantLock.js';
ChainStore.prototype.getInstantLock = _ChainStore_getInstantLock;
import _ChainStore_getTransaction from './methods/getTransaction.js';
ChainStore.prototype.getTransaction = _ChainStore_getTransaction;

import _ChainStore_importAddress from './methods/importAddress.js';
ChainStore.prototype.importAddress = _ChainStore_importAddress;
import _ChainStore_importInstantLock from './methods/importInstantLock.js';
ChainStore.prototype.importInstantLock = _ChainStore_importInstantLock;
import _ChainStore_importTransaction from './methods/importTransaction.js';
ChainStore.prototype.importTransaction = _ChainStore_importTransaction;

export default ChainStore;
const EventEmitter = require('events');

const {
  Transaction, BlockHeader,
} = require('@dashevo/dashcore-lib');
const CONSTANTS = require('../../CONSTANTS');

const SCHEMA = {
  headersMetadata: {
    '*': {
      height: 'number',
      time: 'number',
    },
  },
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
      blockHeight: 0,
      blockHeaders: [],
      headersMetadata: new Map(),
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

  updateHeadersMetadata(headers, tipHeight) {
    headers.forEach((header, index) => {
      if (this.state.headersMetadata.get(header.hash)) {
        throw new Error(`Header ${header.hash} already exists`);
      }

      this.state.headersMetadata.set(header.hash, {
        height: tipHeight - headers.length + index + 1,
        time: header.time,
      });
    });
  }

  clearHeadersMetadata() {
    this.state.headersMetadata.clear();
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

  // TODO(spv): rm?
  set chainHeight(height) {
    if (height < this.state.blockHeight) {
      throw new Error(`Chain height value ${height} is lower than current value ${this.state.blockHeight}`);
    }

    this.state.blockHeight = height;
  }

  get chainHeight() {
    return this.state.blockHeight;
  }

  updateChainHeight(height) {
    if (height < this.state.blockHeight) {
      throw new Error(`Chain height value ${height} is lower than current value ${this.state.blockHeight}`);
    }

    this.state.blockHeight = height;
  }
}

ChainStore.prototype.SCHEMA = SCHEMA;

ChainStore.prototype.considerTransaction = require('./methods/considerTransaction');

ChainStore.prototype.exportState = require('./methods/exportState');
ChainStore.prototype.importState = require('./methods/importState');

ChainStore.prototype.getAddress = require('./methods/getAddress');
ChainStore.prototype.getAddresses = require('./methods/getAddresses');

ChainStore.prototype.getBlockHeader = require('./methods/getBlockHeader');
ChainStore.prototype.getInstantLock = require('./methods/getInstantLock');
ChainStore.prototype.getTransaction = require('./methods/getTransaction');

ChainStore.prototype.importAddress = require('./methods/importAddress');
ChainStore.prototype.importInstantLock = require('./methods/importInstantLock');
ChainStore.prototype.importTransaction = require('./methods/importTransaction');

module.exports = ChainStore;

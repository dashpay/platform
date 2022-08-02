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

    this.state = {
      fees: {
        minRelay: -1,
      },
      lastSyncedHeaderHeight: -1, // TODO: make sure it's -1, it is important for further math
      blockHeight: 0,
      blockHeaders: [],
      headersMetadata: {},
      transactions: new Map(),
      instantLocks: new Map(),
      addresses: new Map(),
    };

    this.headersToKeep = CONSTANTS.STORAGE.headersToKeep;
  }

  getTransactions() {
    return this.state.transactions;
  }

  setBlockHeaders(headers) {
    this.state.blockHeaders = headers;
  }

  // TODO: write tests
  updateHeadersMetadata(headers, tipHeight) {
    headers.forEach((header, index) => {
      if (this.state.headersMetadata[header.hash]) {
        throw new Error(`Header ${header.hash} already exists`);
      }

      Object.assign(this.state.headersMetadata, {
        [header.hash]: {
          height: tipHeight - headers.length + index + 1,
          time: header.time,
        },
      });
    });
    // console.log('Headers metadata', Object.keys(this.state.headersMetadata).length);
  }

  updateLastSyncedHeaderHeight(height) {
    if (height < this.state.lastSyncedHeaderHeight) {
      throw new Error('Cannot update lastSyncedHeaderHeight to a lower value');
    }

    this.state.lastSyncedHeaderHeight = height;
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
ChainStore.prototype.importBlockHeader = require('./methods/importBlockHeader');
ChainStore.prototype.importInstantLock = require('./methods/importInstantLock');
ChainStore.prototype.importTransaction = require('./methods/importTransaction');

module.exports = ChainStore;

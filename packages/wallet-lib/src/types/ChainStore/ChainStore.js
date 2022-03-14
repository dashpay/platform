const EventEmitter = require('events');

const {
  InstantLock, Transaction, BlockHeader, MerkleBlock,
} = require('@dashevo/dashcore-lib');

const SCHEMA = {
  '*': {
    blockHeaders: {
      '*': BlockHeader,
    },
    merkleBlocks: {
      '*': MerkleBlock,
    },
    transactions: {
      '*': Transaction,
    },
    instantLocks: {
      '*': InstantLock,
    },
    txMetadata: {
      '*': {
        blockHash: 'string',
        height: 'number',
      },
    },
    usedAddresses: {
      '*': {
        transactions: {
          '*': 'string',
        },
        utxos: {
          '*': Transaction.Output,
        },
        balance: 'number',
      },
    },
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
      blockHeight: 0,
      blockHeaders: new Map(),
      transactions: new Map(),
      instantLocks: new Map(),
      addresses: new Map(),
    };
  }
}

ChainStore.SCHEMA = SCHEMA;

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

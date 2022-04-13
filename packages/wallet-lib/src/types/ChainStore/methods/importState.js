const {BlockHeader, Transaction} = require('@dashevo/dashcore-lib')
const castItemTypes = require('../../../utils/castItemTypes')

const SCHEMA = {
  blockHeaders: {
    '*': function (hex) {
      return new BlockHeader(Buffer.from(hex, 'hex'));
    },
  },
  transactions: {
    '*': Transaction,
  },
  txMetadata: {
    '*': {
      blockHash: 'string',
      height: 'number',
    },
  },
};

function importState(state) {
  try {
    castItemTypes(state, SCHEMA)
  } catch (e) {
    console.error(e)
  }

  const {
    blockHeaders,
    transactions,
    txMetadata,
  } = state;

  Object.values(blockHeaders).forEach((blockHeader) => {
    this.importBlockHeader(blockHeader);
  });

  Object.keys(transactions).forEach((hash) => {
    const tx = transactions[hash];
    const metadata = txMetadata[hash];
    this.importTransaction(tx, metadata);
  });
}

module.exports = importState;

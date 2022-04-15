function exportState() {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    fees,
  } = state;

  const serializedState = {
    blockHeaders: {},
    transactions: {},
    instantLocks: {},
    txMetadata: {},
    fees: {},
  };

  [...blockHeaders.entries()].forEach(([blockHeaderHash, blockHeader]) => {
    serializedState.blockHeaders[blockHeaderHash] = blockHeader.toString();
  });

  [...transactions.entries()].forEach(([transactionHash, { transaction, metadata }]) => {
    if (metadata && metadata.height) {
      serializedState.transactions[transactionHash] = transaction.toString();
      serializedState.txMetadata[transactionHash] = metadata;
    }
  });

  serializedState.fees.minRelay = fees.minRelay

  return serializedState;
}

module.exports = exportState;

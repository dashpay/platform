function exportState() {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    blockHeight,
  } = state;

  const serializedState = {
    blockHeaders: {},
    transactions: {},
    instantLocks: {},
    txMetadata: {},
  };

  const reorgSafeHeight = blockHeight - 6;

  [...blockHeaders.entries()].forEach(([blockHeaderHash, blockHeader]) => {
    serializedState.blockHeaders[blockHeaderHash] = blockHeader.toString();
  });

  [...transactions.entries()].forEach(([transactionHash, { transaction, metadata }]) => {
    if (metadata && metadata.height && metadata.height <= reorgSafeHeight) {
      serializedState.transactions[transactionHash] = transaction.toString();
      serializedState.txMetadata[transactionHash] = metadata;
    }
  });

  return serializedState;
}

module.exports = exportState;

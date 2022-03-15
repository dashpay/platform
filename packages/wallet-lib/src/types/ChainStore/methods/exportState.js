function exportState() {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    instantLocks,
  } = state;

  const serializedState = {
    blockHeaders: {},
    transactions: {},
    instantLocks: {},
    txMetadata: {},
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

  [...instantLocks.entries()].forEach(([transactionHash, instantLock]) => {
    serializedState.instantLocks[transactionHash] = instantLock.toString();
  });

  return serializedState;
}

module.exports = exportState;

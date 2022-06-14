function exportState(lastKnownBlock) {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    blockHeight,
    fees,
  } = state;

  const serializedState = {
    blockHeaders: {},
    transactions: {},
    txMetadata: {},
    fees: {},
  };

  let reorgSafeHeight = Infinity;

  if (blockHeight) {
    reorgSafeHeight = blockHeight - 6;
  }

  // TODO: temporary construction to control saving progress
  let saveHeight = reorgSafeHeight;

  if (lastKnownBlock < saveHeight) {
    saveHeight = lastKnownBlock;
  }

  [...blockHeaders.entries()].forEach(([blockHeaderHash, blockHeader]) => {
    serializedState.blockHeaders[blockHeaderHash] = blockHeader.toString();
  });

  [...transactions.entries()].forEach(([transactionHash, { transaction, metadata }]) => {
    if (metadata && metadata.height && metadata.height <= saveHeight) {
      serializedState.transactions[transactionHash] = transaction.toString();
      serializedState.txMetadata[transactionHash] = metadata;
    }
  });

  serializedState.fees.minRelay = fees.minRelay;

  return serializedState;
}

module.exports = exportState;

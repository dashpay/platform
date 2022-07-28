function exportState(lastKnownBlock) {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    blockHeight,
    fees,
    headersMetadata,
    lastSyncedHeaderHeight,
  } = state;

  const serializedState = {
    blockHeaders: [],
    transactions: {},
    txMetadata: {},
    fees: {},
    headersMetadata,
    lastSyncedHeaderHeight,
  };

  let reorgSafeHeight = Infinity;

  if (blockHeight) {
    reorgSafeHeight = blockHeight - 6;
  }

  // TODO: control reorg safe height for headers

  // Object.assign(serializedState, {
  //   lastSyncedHeaderHeight: lastSyncedHeaderHeight > reorgSafeHeight
  //     ? reorgSafeHeight
  //     : lastSyncedHeaderHeight,
  // });

  // TODO: temporary construction to control saving progress
  let saveHeight = reorgSafeHeight;

  if (lastKnownBlock < saveHeight) {
    saveHeight = lastKnownBlock;
  }

  serializedState.blockHeaders = blockHeaders.map((header) => header.toString());

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

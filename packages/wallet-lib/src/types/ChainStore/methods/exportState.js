function exportState(lastKnownBlock) {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    blockHeight,
    fees,
    headersMetadata,
    lastSyncedHeaderHeight, // TODO: ensure it's saved with safeHeight in mind
  } = state;

  const serializedState = {
    blockHeaders: [],
    transactions: {},
    txMetadata: {},
    fees: {},
    headersMetadata: Object.fromEntries(headersMetadata),
    lastSyncedHeaderHeight,
    chainHeight: blockHeight,
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
      serializedState.txMetadata[transactionHash] = {
        ...metadata,
        time: metadata.time ? metadata.time.getTime() : -1,
      };
    }
  });

  serializedState.fees.minRelay = fees.minRelay;

  return serializedState;
}

module.exports = exportState;

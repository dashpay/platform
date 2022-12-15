const { STORAGE } = require('../../../CONSTANTS');

function exportState() {
  const { state } = this;
  const {
    blockHeaders,
    transactions,
    chainHeight,
    fees,
    headersMetadata,
    lastSyncedHeaderHeight,
    lastSyncedBlockHeight,
  } = state;

  const serializedState = {
    chainHeight,
    blockHeaders: [],
    transactions: {},
    txMetadata: {},
    fees: {},
  };

  const reorgSafeHeight = chainHeight - STORAGE.REORG_SAFE_BLOCKS_COUNT;
  const lastSyncedHeaderHeightToExport = Math.min(reorgSafeHeight, lastSyncedHeaderHeight);
  const lastSyncedBlockHeightToExport = Math.min(reorgSafeHeight, lastSyncedBlockHeight);

  const headersMetadataToExport = Object.fromEntries(
    [...headersMetadata.entries()]
      .filter(([, { height }]) => height <= lastSyncedHeaderHeightToExport),
  );

  Object.assign(serializedState, {
    lastSyncedHeaderHeight: lastSyncedHeaderHeightToExport,
    lastSyncedBlockHeight: lastSyncedBlockHeightToExport,
    headersMetadata: headersMetadataToExport,
  });

  const ignoredHeadersCount = lastSyncedHeaderHeight - lastSyncedHeaderHeightToExport;
  serializedState.blockHeaders = blockHeaders
    .slice(0, blockHeaders.length - ignoredHeadersCount)
    .map((header) => header.toString());

  [...transactions.entries()].forEach(([transactionHash, { transaction, metadata }]) => {
    if (metadata && metadata.height && metadata.height <= lastSyncedBlockHeightToExport) {
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

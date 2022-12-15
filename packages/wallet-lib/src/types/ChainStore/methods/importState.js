const castStorageItemsTypes = require('../../../utils/castStorageItemsTypes');

function importState(rawState) {
  const stateToImport = { ...rawState };

  const { headersMetadata } = stateToImport;
  // Delete headers metadata from state to import because
  // it's validation very performance sensitive, and we can skip it
  delete stateToImport.headersMetadata;

  const state = castStorageItemsTypes(stateToImport, this.SCHEMA, 'chainStore');

  const {
    blockHeaders,
    transactions,
    txMetadata,
    lastSyncedHeaderHeight,
    lastSyncedBlockHeight,
    chainHeight,
  } = state;

  this.state.blockHeaders = blockHeaders;

  Object.entries(headersMetadata).forEach(([hash, metadata]) => {
    this.state.headersMetadata.set(hash, metadata);
    this.state.hashesByHeight.set(metadata.height, hash);
  });

  this.state.headersMetadata = new Map(Object.entries(headersMetadata));
  this.state.lastSyncedHeaderHeight = lastSyncedHeaderHeight;
  this.state.lastSyncedBlockHeight = lastSyncedBlockHeight;
  this.state.chainHeight = chainHeight;

  Object.keys(transactions).forEach((hash) => {
    const tx = transactions[hash];
    const metadata = txMetadata[hash];
    this.importTransaction(tx, metadata);
  });
}

module.exports = importState;

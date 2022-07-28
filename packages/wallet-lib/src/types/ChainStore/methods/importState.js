const castStorageItemsTypes = require('../../../utils/castStorageItemsTypes');

function importState(rawState) {
  const state = castStorageItemsTypes(rawState, this.SCHEMA);

  const {
    blockHeaders,
    transactions,
    txMetadata,
    headersMetadata,
    lastSyncedHeaderHeight,
  } = state;

  this.state.blockHeaders = blockHeaders;
  this.state.headersMetadata = headersMetadata;
  this.state.lastSyncedHeaderHeight = lastSyncedHeaderHeight;

  Object.keys(transactions).forEach((hash) => {
    const tx = transactions[hash];
    const metadata = txMetadata[hash];
    this.importTransaction(tx, metadata);
  });
}

module.exports = importState;

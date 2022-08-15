const castStorageItemsTypes = require('../../../utils/castStorageItemsTypes');

function importState(rawState) {
  const state = castStorageItemsTypes(rawState, this.SCHEMA, 'chainStore');

  const {
    blockHeaders,
    transactions,
    txMetadata,
    headersMetadata,
    lastSyncedHeaderHeight,
    chainHeight,
  } = state;

  this.state.blockHeaders = blockHeaders;
  this.state.headersMetadata = new Map(Object.entries(headersMetadata));
  this.state.lastSyncedHeaderHeight = lastSyncedHeaderHeight;
  this.state.blockHeight = chainHeight;

  Object.keys(transactions).forEach((hash) => {
    const tx = transactions[hash];
    const metadata = {
      ...txMetadata[hash],
      time: new Date(txMetadata[hash].time),
    };
    this.importTransaction(tx, metadata);
  });
}

module.exports = importState;

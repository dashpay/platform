const castStorageItemsTypes = require('../../../utils/castStorageItemsTypes');

function importState(rawState) {
  const state = castStorageItemsTypes(rawState, this.SCHEMA);

  this.state.walletId = state.walletId;
  this.state.lastKnownBlock = state.lastKnownBlock;
}

module.exports = importState;

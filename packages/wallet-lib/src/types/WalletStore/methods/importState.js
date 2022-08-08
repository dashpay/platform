const castStorageItemsTypes = require('../../../utils/castStorageItemsTypes');

function importState(rawState) {
  const state = castStorageItemsTypes(rawState, this.SCHEMA, 'walletStore');

  this.state.lastKnownBlock = state.lastKnownBlock;
}

module.exports = importState;

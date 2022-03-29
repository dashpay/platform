const { duffsToDash, calculateDuffBalance } = require('../../../utils');

/**
 * Return the confirmed balance of an account.
 * @param {boolean} [displayDuffs=true] - Set the returned format : Duff/dash.
 * @return {number} Balance in dash
 */
function getConfirmedBalance(displayDuffs = true) {
  const {
    walletId, storage, accountPath, network,
  } = this;

  const { addresses } = storage.getWalletStore(walletId).getPathState(accountPath);

  const chainStore = storage.getChainStore(network);
  const totalSat = (calculateDuffBalance(Object.values(addresses), chainStore, 'confirmed'));
  return (displayDuffs) ? totalSat : duffsToDash(totalSat);
}

module.exports = getConfirmedBalance;

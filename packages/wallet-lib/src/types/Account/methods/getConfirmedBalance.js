const { duffsToDash } = require('../../../utils');

/**
 * Return the confirmed balance of an account.
 * @param displayDuffs {boolean} True by default. Set the returned format : Duff/dash.
 * @return {number} Balance in dash
 */
function getConfirmedBalance(displayDuffs = true) {
  const {
    walletId, storage,
  } = this;
  const accountIndex = this.index;
  const totalSat = storage.calculateDuffBalance(walletId, accountIndex, 'confirmed');
  return (displayDuffs) ? totalSat : duffsToDash(totalSat);
}

module.exports = getConfirmedBalance;

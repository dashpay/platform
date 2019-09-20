const { duffsToDash } = require('../../../utils');

/**
 * Return the total balance of unconfirmed utxo
 * @param displayDuffs {boolean} True by default. Set the returned format : Duff/dash.
 * @return {number} Balance in dash
 */
function getUnconfirmedBalance(displayDuffs = true) {
  const {
    walletId, storage, accountIndex,
  } = this;
  const totalSat = storage.calculateDuffBalance(walletId, accountIndex, 'unconfirmed');
  return (displayDuffs) ? totalSat : duffsToDash(totalSat);
}

module.exports = getUnconfirmedBalance;

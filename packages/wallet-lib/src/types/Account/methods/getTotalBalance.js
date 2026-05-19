import { duffsToDash, calculateDuffBalance } from '../../../utils/index.js';

/**
 * Return the total balance of an account (confirmed + unconfirmed).
 * @param displayDuffs {boolean} True by default. Set the returned format : Duff/dash.
 * @return {number} Balance in dash
 */
function getTotalBalance(displayDuffs = true) {
  const {
    walletId, storage, accountPath, network,
  } = this;

  const { addresses } = storage.getWalletStore(walletId).getPathState(accountPath);

  const chainStore = storage.getChainStore(network);

  const totalSat = (calculateDuffBalance(Object.values(addresses), chainStore, 'total'));
  return (displayDuffs) ? totalSat : duffsToDash(totalSat);
}

export default getTotalBalance;
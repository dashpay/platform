const { WALLET_TYPES } = require('../../../CONSTANTS');
/**
 *
 * @param walletId - The wallet Id where to perform the calculation
 * @param accountIndex - The account Index where to perform the calculation
 * @param type {{'confirmed','unconfirmed','total'}} Default: total. Calculate balance by utxo type.
 * @return {number} Balance in duff
 */
module.exports = function calculateDuffBalance(walletId, accountIndex, type = 'total') {
  let totalSat = 0;
  if (walletId === undefined || accountIndex === undefined) {
    throw new Error('Cannot calculate without walletId and accountIndex params');
  }

  const { addresses } = this.getStore().wallets[walletId];
  const subwallets = Object.keys(addresses);
  subwallets.forEach((subwallet) => {
    const paths = Object.keys(addresses[subwallet])
    // We filter out other potential account
      .filter((el) => type === WALLET_TYPES.SINGLE_ADDRESS
            || parseInt(el.split('/')[3], 10) === accountIndex);

    paths.forEach((path) => {
      const address = addresses[subwallet][path];
      const { balanceSat, unconfirmedBalanceSat } = address;
      switch (type) {
        case 'total':
          totalSat += balanceSat + unconfirmedBalanceSat;
          break;
        case 'confirmed':
          totalSat += balanceSat;
          break;
        case 'unconfirmed':
          totalSat += unconfirmedBalanceSat;
          break;
        default:
          throw new Error(`Unexpected balance type. Got ${type}`);
      }
    });
  });

  return totalSat;
};

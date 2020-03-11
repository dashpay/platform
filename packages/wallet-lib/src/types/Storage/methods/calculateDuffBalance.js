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
      .filter((el) => {
        const splitted = el.split('/');
        const index = parseInt((splitted.length === 1) ? splitted[0] : splitted[3], 10);
        return index === accountIndex;
      });

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

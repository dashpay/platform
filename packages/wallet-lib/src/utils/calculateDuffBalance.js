/**
 *
 * @param walletId - The wallet Id where to perform the calculation
 * @param accountIndex - The account Index where to perform the calculation
 * @param type {{'confirmed','unconfirmed','total'}} Default: total. Calculate balance by utxo type.
 * @return {number} Balance in duff
 */
module.exports = function calculateDuffBalance(addresses, chainStore, type = 'total') {
  let totalSat = 0;

  addresses.forEach((address) => {
    const addressData = chainStore.getAddress(address);
    switch (type) {
      case 'total':
        totalSat += addressData.balanceSat + addressData.unconfirmedBalanceSat;
        break;
      case 'confirmed':
        totalSat += addressData.balanceSat;
        break;
      case 'unconfirmed':
        totalSat += addressData.unconfirmedBalanceSat;
        break;
      default:
        throw new Error(`Unexpected balance type. Got ${type}`);
    }
  });
  return totalSat;
};

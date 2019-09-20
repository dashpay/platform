const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');

/**
 * Will update network of a Wallet and child accounts
 * @param network
 * @return {boolean}
 */
module.exports = function updateNetwork(network) {
  if (is.network(network) && network !== this.network) {
    // Used to ensure network exist in Dashcore
    this.network = Dashcore.Networks[network].toString();
    // this.transport.updateNetwork(network);
    if (this.accounts) {
      this.accounts.forEach((acc) => {
        acc.updateNetwork(network);
      });
      return true;
    }
  }
  return false;
};

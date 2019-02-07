const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../utils/index');

/**
 * Will update network of a Wallet and child accounts
 * @param network
 * @return {boolean}
 */
module.exports = function updateNetwork(network) {
  if (is.network(network) && network !== this.network) {
    this.network = Dashcore.Networks[network];
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

const getBIP44Path = require('./getBIP44Path');
const getNetwork = require('./getNetwork');
const { is } = require('../utils');
/**
 * Will update the account network and ask the transport adapter to do the same
 * @param network
 * @return {*}
 */
module.exports = function updateNetwork(network) {
  console.log(`Account network - update to(${network}) - from(${this.network}`);
  if (is.network(network) && network !== this.network) {
    this.BIP44PATH = getBIP44Path(network, this.accountIndex);
    this.network = getNetwork(network);
    this.storage.store.wallets[this.walletId].network = network.toString();
    if (this.transport.isValid) {
      return this.transport.updateNetwork(network);
    }
  }
  return false;
};

const Dashcore = require('@dashevo/dashcore-lib');
const { hasProp } = require('../../../utils');

const { testnet } = Dashcore.Networks;
const createWallet = function (walletId = 'squawk7700', network = testnet.toString(), mnemonic = null, type = null) {
  if (!hasProp(this.store.wallets, walletId)) {
    this.store.wallets[walletId] = {
      accounts: {},
      network,
      mnemonic,
      type,
      blockHeight: 0,
      addresses: {
        external: {},
        internal: {},
        misc: {},
      },
    };
    this.createChain(network);
    return true;
  }
  return false;
};
module.exports = createWallet;

const walletStoreMock = require('../../../fixtures/wallets/c922713eac.json');
const chainStoreMock = require('../../../fixtures/chains/for_wallet_c922713eac.json');
const Storage = require('../../types/Storage/Storage');
const { KeyChainStore, DerivableKeyChain } = require('../../index');

module.exports = (opts = {}) => {
  const { walletId } = walletStoreMock;

  const mockedAccount = {
    walletId,
    index: 0,
    storage: new Storage(),
    accountPath: "m/44'/1'/0'",
    network: 'testnet',
    ...opts,
  };

  mockedAccount.keyChainStore = new KeyChainStore();
  mockedAccount.keyChainStore.addKeyChain(new DerivableKeyChain({
    HDPrivateKey: 'tprv8gpcZgdXPzdXKBjSzieMyfwr6KidKucLiiA9VbCLCx1spyJNd38a5KdjtVuc9bVUNpFM2LdFCrYSyUXHx1RCTdr6qQen1HTECwAZ1p8yqiB',
    lookAheadOpts: {
      'm/0': 40,
      'm/1': 40,
    },
  }), { isMasterKeyChain: true });
  mockedAccount.storage.createWalletStore(walletId);
  mockedAccount.storage.createChainStore('testnet');
  mockedAccount.storage.getWalletStore(walletId).importState(walletStoreMock);
  mockedAccount.storage.getChainStore('testnet').importState(chainStoreMock);

  return mockedAccount;
};

const walletStoreMock = require('./wallet-store.json');
const chainStoreMock = require('./chain-store.json');
const Storage = require('../../../src/types/Storage/Storage');
const { KeyChainStore, DerivableKeyChain } = require('../../../src/index');
const createPathsForTransactions = require('../../../src/types/Account/methods/createPathsForTransactions');
const addPathsToStore = require("../../../src/types/Account/methods/addPathsToStore");
const generateNewPaths = require("../../../src/types/Account/methods/generateNewPaths");
const addDefaultPaths = require("../../../src/types/Account/methods/addDefaultPaths");

module.exports = (opts = {}) => {
    const { walletId } = walletStoreMock;

    const mockedWallet = {
      walletId,
      storage: new Storage(),
      keyChainStore: null
    }

    mockedWallet.storage.createWalletStore(walletId);
    mockedWallet.storage.createChainStore('testnet');

    mockedWallet.keyChainStore = new KeyChainStore();
    mockedWallet.keyChainStore.addKeyChain(new DerivableKeyChain({
      mnemonic: 'apart trip dignity try point rocket damp reflect raw ten normal young',
    }), { isMasterKeyChain: true });

    const walletStore = mockedWallet.storage.getWalletStore(walletId);
    const chainStore = mockedWallet.storage.getChainStore('testnet');
    chainStore.importState(chainStoreMock);

    const mockedAccount0 = {
        walletId,
        index: 0,
        storage: mockedWallet.storage,
        accountPath: "m/44'/1'/0'",
        network: 'testnet',
        walletType: 'hdwallet',
        ...opts,
        addDefaultPaths,
        createPathsForTransactions,
        generateNewPaths,
        addPathsToStore,
        keyChainStore: null
    };

    // This account is not participating directly in the mock.
    // However, we must take it into consideration having in mind that it participates in account_transfer actions
    const mockedAccount1 = {
      walletId,
      network: 'testnet',
      index: 1,
      accountPath: "m/44'/1'/1'",
      keyChainStore: null,
      storage: mockedWallet.storage,
      addDefaultPaths,
      addPathsToStore,
    };

  const accounts = [mockedAccount0, mockedAccount1];
  /**
   * Fill path states for both accounts in wallet store
   */
  accounts.forEach(account => {
    walletStore.createPathState(account.accountPath);
  })

  /**
   * Initialize key chain stores and default derivation paths for accounts
   */
  accounts.forEach(account => {
      account.keyChainStore = mockedWallet.keyChainStore
        .makeChildKeyChainStore(account.accountPath, {
          lookAheadOpts: {
            paths: {
              'm/0': 20,
              'm/1': 20,
            }
          }
        });

      account.addDefaultPaths()
    })


    mockedAccount0.createPathsForTransactions()

    return mockedAccount0;
};

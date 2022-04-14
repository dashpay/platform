const walletStoreMock = require('./wallet-store.json');
const chainStoreMock = require('./chain-store.json');
const Storage = require('../../../src/types/Storage/Storage');
const { KeyChainStore, DerivableKeyChain } = require('../../../src/index');
const createPathsForTransactions = require("../../../src/types/Account/methods/createPathsForTransactions");
const addPathsToStore = require("../../../src/types/Account/methods/addPathsToStore");
const generateNewPaths = require("../../../src/types/Account/methods/generateNewPaths");
const addDefaultPaths = require("../../../src/types/Account/methods/addDefaultPaths");

module.exports = (opts = {}) => {
    const { walletId } = walletStoreMock;

    const mockedAccount = {
        walletId,
        index: 0,
        storage: new Storage(),
        accountPath: 'm/0',
        network: 'testnet',
        walletType: 'privateKey',
        createPathsForTransactions,
        addPathsToStore,
        generateNewPaths,
        addDefaultPaths,
        ...opts,
    };
    mockedAccount.storage.createWalletStore(walletId);
    mockedAccount.storage.createChainStore('testnet');

    const walletStore = mockedAccount.storage.getWalletStore(walletId);
    walletStore.importState(walletStoreMock);
    walletStore.createPathState(mockedAccount.accountPath);

    mockedAccount.storage.getChainStore('testnet').importState(chainStoreMock);

    mockedAccount.keyChainStore = new KeyChainStore();
    mockedAccount.keyChainStore.addKeyChain(new DerivableKeyChain({
        address: 'ycDeuTfs4U77bTb5cq17dame28zdWHVYfk',
        lookAheadOpts: {
            'm/0': 1,
        },
    }), { isMasterKeyChain: true });

    mockedAccount.keyChainStore
      .getMasterKeyChain()
      .getForPath('0', { isWatched: true });
    mockedAccount.addDefaultPaths()

    return mockedAccount;
};

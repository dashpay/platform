const walletStoreMock = require('./wallet-store.json');
const chainStoreMock = require('./chain-store.json');
const Storage = require('../../../src/types/Storage/Storage');
const { KeyChainStore, DerivableKeyChain } = require('../../../src/index');

module.exports = (opts = {}) => {
    const { walletId } = walletStoreMock;

    const mockedAccount = {
        walletId,
        index: 0,
        storage: new Storage(),
        accountPath: "m/44'/1'/0'",
        network: 'testnet',
        walletType: 'hdwallet',
        ...opts,
    };

    mockedAccount.keyChainStore = new KeyChainStore();
    mockedAccount.keyChainStore.addKeyChain(new DerivableKeyChain({
        mnemonic: 'apart trip dignity try point rocket damp reflect raw ten normal young',
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

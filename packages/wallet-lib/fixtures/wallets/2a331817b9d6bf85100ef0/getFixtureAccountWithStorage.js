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
        accountPath: 'm/0',
        network: 'testnet',
        walletType: 'privateKey',
        ...opts,
    };

    mockedAccount.keyChainStore = new KeyChainStore();
    mockedAccount.keyChainStore.addKeyChain(new DerivableKeyChain({
        privateKey: '2a331817b9d6bf85100ef05503d16f9f57c8855dbf13766b2f26c382b716d396',
        lookAheadOpts: {
            'm/0': 1,
        },
    }), { isMasterKeyChain: true });
    mockedAccount.storage.createWalletStore(walletId);
    mockedAccount.storage.createChainStore('testnet');
    mockedAccount.storage.getWalletStore(walletId).importState(walletStoreMock);
    mockedAccount.storage.getChainStore('testnet').importState(chainStoreMock);

    return mockedAccount;
};

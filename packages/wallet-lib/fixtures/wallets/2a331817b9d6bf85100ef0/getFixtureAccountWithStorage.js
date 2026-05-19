import walletStoreMock from './wallet-store.json' with { type: 'json' };
import chainStoreMock from './chain-store.json' with { type: 'json' };
import Storage from '../../../src/types/Storage/Storage.js';
import { KeyChainStore, DerivableKeyChain } from '../../../src/index.js';
import createPathsForTransactions from '../../../src/types/Account/methods/createPathsForTransactions.js';
import addPathsToStore from '../../../src/types/Account/methods/addPathsToStore.js';
import generateNewPaths from '../../../src/types/Account/methods/generateNewPaths.js';
import addDefaultPaths from '../../../src/types/Account/methods/addDefaultPaths.js';

export default (opts = {}) => {
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

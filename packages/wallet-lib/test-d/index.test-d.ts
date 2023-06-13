// Test export of basic types
import {
    Account,
    Wallet,
    ChainStore,
    DerivableKeyChain
} from '..';

// Test wallet constructor
new Wallet({
    passphrase: 'test',
    offlineMode: true,
    debug: false,
    allowSensitiveOperations: true,
    injectDefaultPlugins: true,
    unsafeOptions: {},
    waitForInstantLockTimeout: 1000,
    waitForTxMetadataTimeout: 1000,
    network: 'testnet',
    mnemonic: 'mnemonic',
    seed: 'seed',
    HDPublicKey: "HDPublicKey",
    HDPrivateKey: 'HDPrivateKey',
    privateKey: 'privateKey',
    publicKey: 'publicKey',
    address: 'address',
    storage: {
        purgeOnError: true,
        autoSave: true
    },
    adapter: {},
    plugins: [],
    transport: {},
});

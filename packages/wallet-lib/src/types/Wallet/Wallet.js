const { PrivateKey, Networks } = require('@dashevo/dashcore-lib');

const EventEmitter = require('events');
const _ = require('lodash');
const Storage = require('../Storage/Storage');
const {
  generateNewMnemonic,
} = require('../../utils');

const defaultOptions = {
  debug: false,
  offlineMode: false,
  network: 'testnet',
  plugins: [],
  passphrase: null,
  injectDefaultPlugins: true,
  allowSensitiveOperations: false,
  unsafeOptions: {},
  waitForInstantLockTimeout: 60000,
  waitForTxMetadataTimeout: 540000,
};

const fromMnemonic = require('./methods/fromMnemonic');
const fromPrivateKey = require('./methods/fromPrivateKey');
const fromPublicKey = require('./methods/fromPublicKey');
const fromAddress = require('./methods/fromAddress');
const fromSeed = require('./methods/fromSeed');
const fromHDPublicKey = require('./methods/fromHDPublicKey');
const fromHDPrivateKey = require('./methods/fromHDPrivateKey');
const generateNewWalletId = require('./methods/generateNewWalletId');

const createTransportFromOptions = require('../../transport/createTransportFromOptions');
const ChainSyncMediator = require('./ChainSyncMediator');

/**
 * Instantiate a basic Wallet object,
 * A wallet is able to spawn up all preliminary steps toward the creation of a Account with
 * it's own transactions
 *
 * A wallet can be of multiple types, which some method.
 * Type are attributed in function of opts (mnemonic, seed,...)
 *
 * WALLET_TYPES :
 *     - address : opts.privateKey is provided. Allow to handle a single address object.
 *     - hdwallet : opts.mnemonic or opts.seed is provided. Handle a HD Wallet with it's account.
 */
class Wallet extends EventEmitter {
  /**
   *
   * @param opts
   */
  constructor(opts = defaultOptions) {
    super();
    // Immediate prototype method-composition are used in order to give access in constructor.
    Object.assign(Wallet.prototype, {
      fromMnemonic,
      fromSeed,
      fromHDPrivateKey,
      fromPrivateKey,
      fromPublicKey,
      fromAddress,
      fromHDPublicKey,
      generateNewWalletId,
    });

    this.passphrase = _.has(opts, 'passphrase') ? opts.passphrase : defaultOptions.passphrase;
    this.offlineMode = _.has(opts, 'offlineMode') ? opts.offlineMode : defaultOptions.offlineMode;
    this.debug = _.has(opts, 'debug') ? opts.debug : defaultOptions.debug;
    this.allowSensitiveOperations = _.has(opts, 'allowSensitiveOperations') ? opts.allowSensitiveOperations : defaultOptions.allowSensitiveOperations;
    this.injectDefaultPlugins = _.has(opts, 'injectDefaultPlugins') ? opts.injectDefaultPlugins : defaultOptions.injectDefaultPlugins;
    this.unsafeOptions = _.has(opts, 'unsafeOptions') ? opts.unsafeOptions : defaultOptions.unsafeOptions;
    this.waitForInstantLockTimeout = _.has(opts, 'waitForInstantLockTimeout') ? opts.waitForInstantLockTimeout : defaultOptions.waitForInstantLockTimeout;
    this.waitForTxMetadataTimeout = _.has(opts, 'waitForTxMetadataTimeout') ? opts.waitForTxMetadataTimeout : defaultOptions.waitForTxMetadataTimeout;

    // Validate network
    const networkName = _.has(opts, 'network') ? opts.network.toString() : defaultOptions.network;
    const network = Networks.get(networkName);

    if (!network) {
      throw new Error(`Invalid network: ${network}`);
    }

    this.network = network.toString();

    let createdFromNewMnemonic = false;
    if ('mnemonic' in opts) {
      let { mnemonic } = opts;
      if (mnemonic === null) {
        mnemonic = generateNewMnemonic();
        createdFromNewMnemonic = true;
      }
      this.fromMnemonic(mnemonic, this.network, this.passphrase);
    } else if ('seed' in opts) {
      this.fromSeed(opts.seed, this.network);
    } else if ('HDPrivateKey' in opts) {
      this.fromHDPrivateKey(opts.HDPrivateKey);
    } else if ('privateKey' in opts) {
      this.fromPrivateKey((opts.privateKey === null)
        ? new PrivateKey(network).toString()
        : opts.privateKey, this.network);
    } else if ('publicKey' in opts) {
      this.fromPublicKey(opts.publicKey, this.network);
    } else if ('HDPublicKey' in opts) {
      this.fromHDPublicKey(opts.HDPublicKey);
    } else if ('address' in opts) {
      this.fromAddress(opts.address, this.network);
    } else {
      this.fromMnemonic(generateNewMnemonic());
      createdFromNewMnemonic = true;
    }

    // Notice : Most of the time, wallet id is deterministic
    this.generateNewWalletId();

    this.storage = new Storage({
      rehydrate: true,
      autosave: true,
    });

    this.storage.application.network = this.network;
    this.storage.configure({
      adapter: opts.adapter,
      walletId: this.walletId,
      network: this.network,
    });

    if (createdFromNewMnemonic) {
      // As it is pretty complicated to pass any of wallet options
      // to a specific plugin, using `store` as an options mediator
      // is easier.

      this.storage.application.syncOptions = {
        skipSynchronization: true,
      };

      if (this.unsafeOptions.skipSynchronizationBeforeHeight) {
        throw new Error('"unsafeOptions.skipSynchronizationBeforeHeight" will have no effect because wallet has been'
          + ' created from the new mnemonic');
      }
    } else if (this.unsafeOptions.skipSynchronizationBeforeHeight) {
      this.storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: this.unsafeOptions.skipSynchronizationBeforeHeight,
      };
    }

    const plugins = opts.plugins || defaultOptions.plugins;
    this.plugins = {};
    // eslint-disable-next-line no-return-assign
    plugins.map((item) => this.plugins[item.name] = item);

    // Handle import of cache
    if (opts.cache) {
      if (opts.cache.transactions) {
        this.storage.importTransactions(opts.cache.transactions);
      }
      if (opts.cache.addresses) {
        this.storage.importAddresses(opts.cache.addresses, this.walletId);
      }
    }

    if (!this.offlineMode) {
      if (opts.transport && opts.transport.network) {
        throw new Error('Please use Wallet\'s "network" option');
      }

      if (!opts.transport) {
        // eslint-disable-next-line no-param-reassign
        opts.transport = {};
      }

      // TODO: networkName might be different from this.network.
      // E.g. devnet networkName equals to testnet. Figure out what are the implications

      // eslint-disable-next-line no-param-reassign
      opts.transport.network = networkName;

      this.transport = createTransportFromOptions(opts.transport);
    }

    this.accounts = [];
    this.interface = opts.interface;
    // Suppressed global require to avoid cyclic dependencies
    // eslint-disable-next-line global-require
    const Identities = require('../Identities/Identities');
    this.identities = new Identities(this);
    this.savedBackup = false; // TODO: When true, we delete mnemonic from internals

    this.chainSyncMediator = new ChainSyncMediator();
  }
}

Wallet.prototype.createAccount = require('./methods/createAccount');
Wallet.prototype.disconnect = require('./methods/disconnect');
Wallet.prototype.getAccount = require('./methods/getAccount');
Wallet.prototype.generateNewWalletId = generateNewWalletId;
Wallet.prototype.exportWallet = require('./methods/exportWallet');
Wallet.prototype.sweepWallet = require('./methods/sweepWallet');
Wallet.prototype.dumpStorage = require('./methods/dumpStorage');

module.exports = Wallet;

const Dashcore = require('@dashevo/dashcore-lib');
const _ = require('lodash');
const Storage = require('../Storage/Storage');
const {
  generateNewMnemonic,
  is,
} = require('../../utils');

const Transporter = require('../../transports/Transporter');

const defaultOptions = {
  offlineMode: false,
  network: 'testnet',
  plugins: [],
  passphrase: null,
  injectDefaultPlugins: true,
  allowSensitiveOperations: false,
};

const fromMnemonic = require('./methods/fromMnemonic');
const fromPrivateKey = require('./methods/fromPrivateKey');
const fromSeed = require('./methods/fromSeed');
const fromHDPublicKey = require('./methods/fromHDPublicKey');
const fromHDPrivateKey = require('./methods/fromHDPrivateKey');
const generateNewWalletId = require('./methods/generateNewWalletId');

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
class Wallet {
  /**
   *
   * @param opts
   */
  constructor(opts = defaultOptions) {
    // Immediate prototype method-composition are used in order to give access in constructor.
    Object.assign(Wallet.prototype, {
      fromMnemonic,
      fromSeed,
      fromHDPrivateKey,
      fromPrivateKey,
      fromHDPublicKey,
      generateNewWalletId,
    });

    const network = _.has(opts, 'network') ? opts.network.toString() : defaultOptions.network;
    const passphrase = _.has(opts, 'passphrase') ? opts.passphrase : defaultOptions.passphrase;
    this.passphrase = passphrase;
    this.offlineMode = _.has(opts, 'offlineMode') ? opts.offlineMode : defaultOptions.offlineMode;
    this.allowSensitiveOperations = _.has(opts, 'allowSensitiveOperations') ? opts.allowSensitiveOperations : defaultOptions.allowSensitiveOperations;
    this.injectDefaultPlugins = _.has(opts, 'injectDefaultPlugins') ? opts.injectDefaultPlugins : defaultOptions.injectDefaultPlugins;

    if (!(is.network(network))) throw new Error('Expected a valid network (typeof String)');
    if (!Dashcore.Networks[network]) {
      throw new Error(`Un-handled network: ${network}`);
    }
    this.network = Dashcore.Networks[network].toString();

    if ('mnemonic' in opts) {
      this.fromMnemonic((opts.mnemonic === null) ? generateNewMnemonic() : opts.mnemonic);
    } else if ('seed' in opts) {
      this.fromSeed(opts.seed);
    } else if ('HDPrivateKey' in opts) {
      this.fromHDPrivateKey(opts.HDPrivateKey);
    } else if ('privateKey' in opts) {
      this.fromPrivateKey(opts.privateKey);
    } else if ('HDPublicKey' in opts) {
      this.fromHDPublicKey(opts.HDPublicKey);
    } else {
      this.fromMnemonic(generateNewMnemonic());
    }

    // Notice : Most of the time, wallet id is deterministic
    this.generateNewWalletId();

    this.storage = new Storage({
      rehydrate: true,
      autosave: true,
      network,
    });

    this.storage.configure({
      adapter: opts.adapter,
    });

    this.store = this.storage.store;
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
    if (this.offlineMode) {
      this.transport = { isValid: false };
    } else {
      this.transport = (opts.transport) ? new Transporter(opts.transport) : new Transporter();
    }
    this.accounts = [];
    this.interface = opts.interface;
    this.savedBackup = false; // TODO: When true, we delete mnemonic from internals
  }
}

Wallet.prototype.createAccount = require('./methods/createAccount');
Wallet.prototype.disconnect = require('./methods/disconnect');
Wallet.prototype.getAccount = require('./methods/getAccount');
Wallet.prototype.generateNewWalletId = generateNewWalletId;
Wallet.prototype.exportWallet = require('./methods/exportWallet');

module.exports = Wallet;

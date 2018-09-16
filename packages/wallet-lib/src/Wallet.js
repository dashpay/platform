const Dashcore = require('@dashevo/dashcore-lib');
const InMem = require('./adapters/InMem');
const Storage = require('./Storage');
const KeyChain = require('./KeyChain');
// const { Registration, TopUp } = Dashcore.Transaction.SubscriptionTransactions;
const {
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
  is,
} = require('./utils/index');

const Account = require('./Account');
const Transporter = require('./transports/Transporter');

const defaultOptions = {
  network: 'testnet',
};

/**
 * Instantiate a basic Wallet object,
 * A wallet is able to spawn up all preliminary steps toward the creation of a Account with
 * it's own transactions
 */
class Wallet {
  /**
   *
   * @param opts
   */
  constructor(opts = defaultOptions) {
    let HDPrivateKey = null;
    let passphrase = null;
    let mnemonic = null;

    if (!(opts.network && is.network(opts.network))) throw new Error('Expected a valid network (typeof Network or String)');
    this.network = Dashcore.Networks[opts.network];
    // eslint-disable-next-line prefer-destructuring
    if (opts.passphrase) passphrase = opts.passphrase;


    if ('mnemonic' in opts) {
      if (!is.mnemonic(opts.mnemonic)) throw new Error('Expected a valid mnemonic (typeof String or Mnemonic)');
      // eslint-disable-next-line prefer-destructuring
      mnemonic = opts.mnemonic;
      HDPrivateKey = mnemonicToHDPrivateKey(opts.mnemonic, this.network, passphrase);
    } else if ('seed' in opts) {
      if (!is.seed(opts.seed)) throw new Error('Expected a valid seed (typeof HDPrivateKey or String)');
      HDPrivateKey = opts.seed;
      mnemonic = null; // todo : verify if possible to change from HDPrivateKey to Mnemonic back
    } else {
      mnemonic = generateNewMnemonic();
      HDPrivateKey = mnemonicToHDPrivateKey(mnemonic, this.network, passphrase);
    }

    this.walletId = (mnemonic) ? mnemonicToWalletId(mnemonic) : mnemonicToWalletId(HDPrivateKey);
    this.adapter = (opts.adapter) ? opts.adapter : new InMem();
    this.adapter.config();

    this.storage = new Storage({
      adapter: this.adapter,
      walletId: this.walletId,
      network: this.network,
      mnemonic,
    });
    this.store = this.storage.store;

    // Handle import of cache
    if (opts.cache) {
      if (opts.cache.transactions) {
        this.storage.importTransactions(opts.cache.transactions);
      }
      if (opts.cache.addresses) {
        this.storage.importAddresses(opts.cache.addresses, this.walletId);
      }
    }

    this.transport = (opts.transport) ? new Transporter(opts.transport) : new Transporter();

    this.accounts = [];
    this.HDPrivateKey = HDPrivateKey;
    this.keychain = new KeyChain(this.HDPrivateKey);
    this.mnemonic = mnemonic; // We keep it only for the export function..
    this.interface = opts.interface;
    this.savedBackup = false; // When true, we delete mnemonic from internals
  }

  // TODO : Add tests
  updateNetwork(network) {
    if (is.network(network) && network !== this.network) {
      this.network = Dashcore.Networks[network];
      this.transport.updateNetwork(network);
      this.accounts.forEach((acc) => {
        acc.updateNetwork(network);
      });
      return true;
    }
    return false;
  }

  /**
   * Will derivate to a new account.
   * @param {object} account options
   * @return {account} - account object
   */
  createAccount(accountOpts) {
    // if(this.accounts[])
    // Auto-populate this.accounts, probably not what we want ?
    return new Account(this, accountOpts);
  }

  /**
   * Get a specific account per accountIndex
   * @param accountIndex - Default: 0, set a specific index to get
   * @param accountOpts - If we can't get, we create passing these arg to createAccount method
   * @return {*|account}
   */
  getAccount(accountIndex = 0, accountOpts) {
    const acc = this.accounts.filter(el => el.accountIndex === accountIndex);
    const opts = Object.assign({ accountIndex }, accountOpts);
    return (acc[0]) || this.createAccount(opts);
  }

  /**
   * Export the wallet (mnemonic)
   * @param toHDPrivateKey - Default: false - Allow to return to a HDPrivateKey type
   * @return {Mnemonic|HDPrivateKey}
   */
  exportWallet(toHDPrivateKey = false) {
    function exportMnemonic(mnemonic) {
      if (mnemonic === null) throw new Error('Wallet was not initiated with a mnemonic, can\'t export it');
      return mnemonic.toString();
    }
    return (toHDPrivateKey) ? this.HDPrivateKey : exportMnemonic(this.mnemonic);
  }

  /**
   * Disconnect all the storage worker and process all account to disconnect their endpoint too.
   */
  disconnect() {
    if (this.storage) {
      this.storage.stopWorker();
    }
    if (this.accounts) {
      const accountPath = Object.keys(this.accounts);
      accountPath.forEach((path) => {
        this.accounts[path].disconnect();
      });
    }
  }
}

module.exports = Wallet;

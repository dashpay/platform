// Default winston transport requires setImmediate to work, so
// polyfill included here. Making it work with webpack is rather tricky, so it is used as per
// documentation: https://github.com/YuzuJS/setImmediate#usage
require('setimmediate');
const Account = require('./types/Account/Account');
const ChainStore = require('./types/ChainStore/ChainStore');
const Identities = require('./types/Identities/Identities');
const DerivableKeyChain = require('./types/DerivableKeyChain/DerivableKeyChain');
const KeyChainStore = require('./types/KeyChainStore/KeyChainStore');
const Storage = require('./types/Storage/Storage');
const Wallet = require('./types/Wallet/Wallet');
const WalletStore = require('./types/WalletStore/WalletStore');
const EVENTS = require('./EVENTS');
const CONSTANTS = require('./CONSTANTS');
const utils = require('./utils');
const plugins = require('./plugins');

module.exports = {
  Account,
  ChainStore,
  Identities,
  DerivableKeyChain,
  KeyChainStore,
  Storage,
  Wallet,
  WalletStore,
  EVENTS,
  CONSTANTS,
  utils,
  plugins,
};

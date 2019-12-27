const Wallet = require('./types/Wallet/Wallet');
const Account = require('./types/Account/Account');
const KeyChain = require('./types/KeyChain/KeyChain');
const EVENTS = require('./EVENTS');
const CONSTANTS = require('./CONSTANTS');
const utils = require('./utils');
const plugins = require('./plugins');

module.exports = {
  Wallet,
  Account,
  KeyChain,
  EVENTS,
  CONSTANTS,
  utils,
  plugins,
};

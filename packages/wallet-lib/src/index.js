const Wallet = require('./types/Wallet/Wallet');
const Account = require('./types/Account/Account');
const Identities = require('./types/Identities/Identities');
const KeyChain = require('./types/KeyChain/KeyChain');
const EVENTS = require('./EVENTS');
const CONSTANTS = require('./CONSTANTS');
const utils = require('./utils');
const plugins = require('./plugins');

module.exports = {
  Wallet,
  Account,
  Identities,
  KeyChain,
  EVENTS,
  CONSTANTS,
  utils,
  plugins,
};

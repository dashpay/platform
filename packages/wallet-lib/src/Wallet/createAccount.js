const { WALLET_TYPES } = require('../CONSTANTS');
const Account = require('../Account/Account');
/**
 * Will derivate to a new account.
 * @param {object} accountOpts - options to pass, will autopopulate some
 * @return {account} - account object
 */
function createAccount(accountOpts) {
  const { injectDefaultPlugins, plugins, allowSensitiveOperations } = this;
  const baseOpts = { injectDefaultPlugins, allowSensitiveOperations, plugins };
  if (this.type === WALLET_TYPES.SINGLE_ADDRESS) { baseOpts.privateKey = this.privateKey; }
  const opts = Object.assign(baseOpts, accountOpts);
  return new Account(this, opts);
}
module.exports = createAccount;

const { WALLET_TYPES } = require('../CONSTANTS');
/**
 * Will derivate to a new account.
 * @param {object} accountOpts - options to pass, will autopopulate some
 * @return {account} - account object
 */
function createAccount(accountOpts) {
  /**
   *   Wallet.createAccount calls Account that depends on Wallet.
   *   In order to avoid a cyclic dependency issue we put this require here and
   *   disable eslint global require for next line
   */
  // eslint-disable-next-line global-require
  const Account = require('../Account/Account.js');

  const { injectDefaultPlugins, plugins, allowSensitiveOperations } = this;
  const baseOpts = { injectDefaultPlugins, allowSensitiveOperations, plugins };
  if (this.type === WALLET_TYPES.SINGLE_ADDRESS) { baseOpts.privateKey = this.privateKey; }
  const opts = Object.assign(baseOpts, accountOpts);
  return new Account(this, opts);
}
module.exports = createAccount;

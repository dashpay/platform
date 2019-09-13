const _ = require('lodash');
const { is } = require('../utils');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  index: 0,
};
/**
 * Get a specific account per accountIndex
 * @param accountOpts - If the account doesn't exist yet, we create it passing these options
 * @param accountOpts.index - Default: 0, set a specific index to get
 * @return {*|account}
 */

function getAccount(accountOpts = JSON.parse(JSON.stringify(_defaultOpts))) {
  const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
  const accountIndex = (_.has(accountOpts, 'index') && is.num(accountOpts.index))
    ? accountOpts.index
    : defaultOpts.index;

  const acc = this.accounts.filter((el) => el.accountIndex === accountIndex);
  const baseOpts = { accountIndex };

  const opts = Object.assign(baseOpts, accountOpts);
  const account = (acc[0]) || this.createAccount(opts);
  account.storage.attachEvents(account.events);
  return account;
}
module.exports = getAccount;

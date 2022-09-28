const _ = require('lodash');
const { is } = require('../../../utils');
const EVENTS = require('../../../EVENTS');

/**
 * Get a specific account per accounts index
 * @param accountOpts - If the account doesn't exist yet, we create it passing these options
 * @param accountOpts.index - Default: 0, set a specific index to get
 * @return {Account}
 */

async function getAccount(accountOpts = {}) {
  if (!this.storage.configured) {
    await new Promise((resolve) => this.storage.once(EVENTS.CONFIGURED, resolve));
  }

  if (is.num(accountOpts)) {
    throw new Error('getAccount expected index integer to be a property of accountOptions');
  }
  const defaultIndex = 0;

  const accountIndex = (_.has(accountOpts, 'index') && is.num(accountOpts.index))
    ? accountOpts.index
    : defaultIndex;

  const acc = this.accounts.filter((el) => el.index === accountIndex);
  const baseOpts = { index: accountIndex };

  const opts = Object.assign(baseOpts, accountOpts);
  return (acc[0]) || this.createAccount(opts);
}

module.exports = getAccount;

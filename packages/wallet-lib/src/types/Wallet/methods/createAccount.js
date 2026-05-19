import { WALLET_TYPES } from '../../../CONSTANTS.js';
import EVENTS from '../../../EVENTS.js';
import Account from '../../Account/Account.js';
/**
 * Will derivate to a new account.
 * @param {object} accountOpts - options to pass, will autopopulate some
 * @return {Account} - account object
 */
async function createAccount(accountOpts) {
  if (!this.storage.configured) {
    await new Promise((resolve) => { this.storage.once(EVENTS.CONFIGURED, resolve); });
  }

  const {
    injectDefaultPlugins, debug, plugins, allowSensitiveOperations,
  } = this;
  const baseOpts = {
    injectDefaultPlugins, debug, allowSensitiveOperations, plugins,
  };
  if (this.walletType === WALLET_TYPES.SINGLE_ADDRESS) { baseOpts.privateKey = this.privateKey; }
  const opts = Object.assign(baseOpts, accountOpts);

  const account = new Account(this, opts);

  // Add default derivation paths
  account.addDefaultPaths();

  // Issue additional derivation paths in case we have transactions in the store
  // at the moment of initialization (from persistent storage)
  account.createPathsForTransactions();

  // Add block headers from storage into the SPV chain if there are any
  const chainStore = this.storage.getDefaultChainStore();
  const { blockHeaders, lastSyncedHeaderHeight } = chainStore.state;
  if (!this.offlineMode && blockHeaders.length > 0) {
    const { blockHeadersProvider } = this.transport.client;
    blockHeadersProvider.initializeChainWith(blockHeaders, lastSyncedHeaderHeight);
  }

  this.accounts.push(account);

  try {
    if (opts.synchronize) {
      await account.init(this);
    }

    return account;
  } catch (e) {
    await account.disconnect();
    throw e;
  }
}
export default createAccount;
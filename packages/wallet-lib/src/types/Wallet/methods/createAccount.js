const { WALLET_TYPES } = require('../../../CONSTANTS');
const EVENTS = require('../../../EVENTS');
const deriveBlockHeadersResumeContext = require('../../ChainStore/deriveBlockHeadersResumeContext');
/**
 * Will derivate to a new account.
 * @param {object} accountOpts - options to pass, will autopopulate some
 * @return {Account} - account object
 */
async function createAccount(accountOpts) {
  if (!this.storage.configured) {
    await new Promise((resolve) => { this.storage.once(EVENTS.CONFIGURED, resolve); });
  }

  /**
   *   Wallet.createAccount calls Account that depends on Wallet.
   *   In order to avoid a cyclic dependency issue we put this require here and
   *   disable eslint global require for next line
   */
  // eslint-disable-next-line global-require
  const Account = require('../../Account/Account');

  const {
    injectDefaultPlugins, debug, plugins, allowSensitiveOperations,
  } = this;
  const baseOpts = {
    injectDefaultPlugins, debug, allowSensitiveOperations, plugins,
  };
  if (this.walletType === WALLET_TYPES.SINGLE_ADDRESS) { baseOpts.privateKey = this.privateKey; }
  const opts = Object.assign(baseOpts, accountOpts);

  if (!this.offlineMode) {
    const chainStore = this.storage.getDefaultChainStore();
    const blockHeadersProvider = this.transport?.client?.blockHeadersProvider;

    if (blockHeadersProvider && !this.blockHeadersProviderInitializationPromise) {
      this.blockHeadersProviderInitializationPromise = (async () => {
        const resumeContext = deriveBlockHeadersResumeContext(chainStore.state);

        if (resumeContext.requiresHeaderStateReset) {
          chainStore.resetBlockHeaders();

          const hasPersistentAutosave = this.storage.autosave
            && this.storage.adapter
            && typeof this.storage.adapter.setItem === 'function';

          if (hasPersistentAutosave) {
            await this.storage.saveState();
          }
        }

        await blockHeadersProvider.initializeChainWith(
          resumeContext.blockHeaders,
          resumeContext.firstHeaderHeight,
        );
      })();
    }

    if (this.blockHeadersProviderInitializationPromise) {
      await this.blockHeadersProviderInitializationPromise;
    }
  }

  const account = new Account(this, opts);

  // Add default derivation paths
  account.addDefaultPaths();

  // Issue additional derivation paths in case we have transactions in the store
  // at the moment of initialization (from persistent storage)
  account.createPathsForTransactions();

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
module.exports = createAccount;

const logger = require('../../logger');
const EVENTS = require('../../EVENTS');
const preparePlugins = require('./_preparePlugins');

// eslint-disable-next-line no-underscore-dangle
async function _initializeAccount(account, userUnsafePlugins) {
  const self = account;

  const accountStore = account.storage
    .getWalletStore(account.walletId)
    .getPathState(account.accountPath);

  const chainStore = account.storage.getChainStore(account.network);

  const issuedPaths = account.keyChainStore
    .getMasterKeyChain()
    .getIssuedPaths();

  issuedPaths.forEach((issuedPath) => {
    accountStore.addresses[issuedPath.path] = issuedPath.address.toString();
    chainStore.importAddress(issuedPath.address.toString());
  });

  // We run faster in offlineMode to speed up the process when less happens.
  const readinessIntervalTime = (account.offlineMode) ? 50 : 200;
  // TODO: perform rejection with a timeout
  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    try {
      // Will sort and inject plugins.
      await preparePlugins(account, userUnsafePlugins);

      self.emit(EVENTS.STARTED, { type: EVENTS.STARTED, payload: null });

      const sendReady = () => {
        if (!self.state.isReady) {
          self.emit(EVENTS.READY, { type: EVENTS.READY, payload: null });
          self.state.isReady = true;
        }
      };
      const sendInitialized = () => {
        if (!self.state.isInitialized) {
          self.emit(EVENTS.INITIALIZED, { type: EVENTS.INITIALIZED, payload: null });
          logger.debug(`Initialized with ${Object.keys(account.plugins.watchers).length} plugins`);
          self.state.isInitialized = true;
        }
      };

      let readyPlugins = 0;
      // eslint-disable-next-line no-param-reassign,consistent-return
      account.readinessInterval = setInterval(() => {
        const watchedPlugins = Object.keys(account.plugins.watchers);
        watchedPlugins.forEach((pluginName) => {
          const watchedPlugin = account.plugins.watchers[pluginName];
          if (watchedPlugin.ready === true && !watchedPlugin.announced) {
            logger.debug(`Initializing - ${readyPlugins}/${watchedPlugins.length} plugins`);
            readyPlugins += 1;
            watchedPlugin.announced = true;
            logger.debug(`Initialized ${pluginName} - ${readyPlugins}/${watchedPlugins.length} plugins`);
          }
        });
        logger.debug(`Initializing - ${readyPlugins}/${watchedPlugins.length} plugins`);
        if (readyPlugins === watchedPlugins.length) {
          // At this stage, our worker are initialized
          sendInitialized();

          // If both of the plugins are present
          // We need to tweak it a little bit to have BIP44 ensuring address
          // while SyncWorker fetch'em on network
          clearInterval(self.readinessInterval);

          if (!account.injectDefaultPlugins) {
            sendReady();
            return resolve(true);
          }

          sendReady();
          return resolve(true);
        }
      }, readinessIntervalTime);
    } catch (e) {
      reject(e);
    }
  });
}

module.exports = _initializeAccount;

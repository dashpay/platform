const _ = require('lodash');
const logger = require('../../logger');
const SyncWorker = require('../../plugins/Workers/SyncWorker/SyncWorker');
const ChainPlugin = require('../../plugins/Plugins/ChainPlugin');
const BIP44Worker = require('../../plugins/Workers/BIP44Worker/BIP44Worker');
const IdentitySyncWorker = require('../../plugins/Workers/IdentitySyncWorker');
const EVENTS = require('../../EVENTS');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { PluginFailedOnStart, WorkerFailedOnExecute, InjectionToPluginUnallowed } = require('../../errors');

// eslint-disable-next-line no-underscore-dangle
async function _initializeAccount(account, userUnsafePlugins) {
  const self = account;
  // We run faster in offlineMode to speed up the process when less happens.
  const readinessIntervalTime = (account.offlineMode) ? 50 : 200;
  // TODO: perform rejection with a timeout
  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    if (account.injectDefaultPlugins) {
      // TODO: Should check in other accounts if a similar is setup already
      // TODO: We want to sort them by dependencies and deal with the await this way
      // await parent if child has it in dependency
      // if not, then is it marked as requiring a first exec
      // if yes add to watcher list.

      try {
        if ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(account.walletType)) {
          // Ideally we should move out from worker to event based
          await account.injectPlugin(BIP44Worker, true);
        }
        if (!account.offlineMode) {
          await account.injectPlugin(ChainPlugin, true);
        }
        if (account.walletType === WALLET_TYPES.SINGLE_ADDRESS) {
          await account.getAddress('0'); // We force what is usually done by the BIP44Worker.
        }
        if (!account.offlineMode) {
          await account.injectPlugin(SyncWorker, true);
          if (account.walletType === WALLET_TYPES.HDWALLET) {
            await account.injectPlugin(IdentitySyncWorker, true);
          }
        }
      } catch (err) {
        throw new Error(`Failed to perform standard injections with reason: ${err.message}`);
      }
    }

    _.each(userUnsafePlugins, (UnsafePlugin) => {
      account.injectPlugin(UnsafePlugin, account.allowSensitiveOperations)
        .catch((e) => {
          if (![
            PluginFailedOnStart,
            WorkerFailedOnExecute,
            InjectionToPluginUnallowed,
          ].includes(e.constructor)) {
            throw new Error(`Failed to inject plugin: ${UnsafePlugin.name}, reason: ${e.message}`);
          }
          return reject(e);
        });
    });

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

    // The need for this function is to pre-generate the address when syncWorker is also doing so
    // We then ensure first syncWorker to do it's job (which will fetch all used address)
    // and only then lookup for remaining needed 20 unused address.
    // Without that, waiting a tick would result similarly.
    const recursivelyGenerateAddresses = async (isSyncWorkerActive = true) => {
      const bip44worker = account.getWorker('BIP44Worker');

      const exec = async () => {
        if (isSyncWorkerActive) {
          const syncWorker = account.getWorker('syncWorker');
          await syncWorker.execute();
        }
        return bip44worker.ensureEnoughAddress();
      };
      if (await exec() !== 0) return recursivelyGenerateAddresses();
      return true;
    };

    // eslint-disable-next-line no-param-reassign,consistent-return
    account.readinessInterval = setInterval(() => {
      const watchedPlugins = Object.keys(account.plugins.watchers);
      let readyPlugins = 0;
      watchedPlugins.forEach((pluginName) => {
        if (account.plugins.watchers[pluginName].ready === true) {
          readyPlugins += 1;
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

        const resultBIP44WorkerSearch = account.hasPlugins(BIP44Worker);
        const resultSyncWorkerSearch = account.hasPlugins(SyncWorker);
        const isSyncWorkerActive = resultSyncWorkerSearch.found;

        if (account.walletType === WALLET_TYPES.SINGLE_ADDRESS) {
          account.generateAddress(0);
          sendReady();
          return resolve(true);
        }

        if (!account.injectDefaultPlugins) {
          sendReady();
          return resolve(true);
        }

        if (!resultBIP44WorkerSearch.found) {
          if (account.walletType === WALLET_TYPES.SINGLE_ADDRESS) {
            sendReady();
            return resolve(true);
          }
          throw new Error('Unable to initialize. BIP44 Worker not found.');
        }
        return recursivelyGenerateAddresses(isSyncWorkerActive)
          .then(() => {
            sendReady();
            return resolve(true);
          })
          .catch((err) => {
            throw new Error(`Unable to generate addresses :${err}`);
          });
      }
    }, readinessIntervalTime);
  });
}

module.exports = _initializeAccount;

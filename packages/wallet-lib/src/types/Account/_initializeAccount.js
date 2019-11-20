const _ = require('lodash');
const logger = require('../../logger');
const SyncWorker = require('../../plugins/Workers/SyncWorker');
const ChainWorker = require('../../plugins/Workers/ChainWorker');
const BIP44Worker = require('../../plugins/Workers/BIP44Worker');
const EVENTS = require('../../EVENTS');
const { WALLET_TYPES } = require('../../CONSTANTS');

// eslint-disable-next-line no-underscore-dangle
async function _initializeAccount(account, userUnsafePlugins) {
  const self = account;
  // We run faster in offlineMode to speed up the process when less happens.
  const readinessIntervalTime = (account.offlineMode) ? 50 : 200;
  return new Promise((res) => {
    if (account.injectDefaultPlugins) {
      // TODO: Should check in other accounts if a similar is setup already
      // TODO: We want to sort them by dependencies and deal with the await this way
      // await parent if child has it in dependency
      // if not, then is it marked as requiring a first exec
      // if yes add to watcher list.

      try {
        if (!account.offlineMode) {
          account.injectPlugin(ChainWorker, true);
        }

        if ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDEXTPUBLIC].includes(account.walletType)) {
          // Ideally we should move out from worker to event based
          account.injectPlugin(BIP44Worker, true);
        }
        if (account.type === WALLET_TYPES.SINGLE_ADDRESS) {
          account.getAddress('0'); // We force what is usually done by the BIP44Worker.
        }
        if (!account.offlineMode) {
          account.injectPlugin(SyncWorker, true);
        }
      } catch (err) {
        logger.error('Failed to perform standard injections', err);
      }
    }

    _.each(userUnsafePlugins, (UnsafePlugin) => {
      try {
        account.injectPlugin(UnsafePlugin, account.allowSensitiveOperations);
      } catch (e) {
        logger.error('Failed to inject plugin:', UnsafePlugin.name);
        logger.error(e);
      }
    });

    self.events.emit(EVENTS.STARTED);

    const sendReady = () => {
      if (!self.isReady) {
        self.events.emit(EVENTS.READY);
        self.isReady = true;
      }
    };
    const sendInitialized = () => {
      if (!self.isInitialized) {
        self.events.emit(EVENTS.INITIALIZED);
        self.isInitialized = true;
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
      const watchedWorkers = Object.keys(account.plugins.watchers);
      let readyWorkers = 0;
      watchedWorkers.forEach((workerName) => {
        if (account.plugins.watchers[workerName].ready === true) {
          readyWorkers += 1;
        }
      });
      if (readyWorkers === watchedWorkers.length) {
        // At this stage, our worker are initialized
        sendInitialized();

        // If both of the plugins are present
        // We need to tweak it a little bit to have BIP44 ensuring address
        // while SyncWorker fetch'em on network
        clearInterval(self.readinessInterval);

        const resultBIP44WorkerSearch = account.hasPlugins(BIP44Worker);
        const resultSyncWorkerSearch = account.hasPlugins(SyncWorker);
        const isSyncWorkerActive = resultSyncWorkerSearch.found;

        if (!resultBIP44WorkerSearch.found) {
          throw new Error('Unable to initialize. BIP44 Worker not found.');
        }
        return recursivelyGenerateAddresses(isSyncWorkerActive)
          .then(() => {
            sendReady();
            return res(true);
          })
          .catch((err) => {
            logger.error('Error', err);
            throw new Error(`Unable to generate addresses :${err}`);
          });
      }
    }, readinessIntervalTime);
  });
}

module.exports = _initializeAccount;

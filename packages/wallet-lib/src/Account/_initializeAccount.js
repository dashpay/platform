const _ = require('lodash');
const SyncWorker = require('../plugins/Workers/SyncWorker');
const ChainWorker = require('../plugins/Workers/ChainWorker');
const BIP44Worker = require('../plugins/Workers/BIP44Worker');
const EVENTS = require('../EVENTS');
const { WALLET_TYPES } = require('../CONSTANTS');

// eslint-disable-next-line no-underscore-dangle
async function _initializeAccount(account, userUnsafePlugins) {
  const self = account;
  return new Promise((res) => {
    if (account.injectDefaultPlugins) {
      // TODO: Should check in other accounts if a similar is setup already
      // TODO: We want to sort them by dependencies and deal with the await this way
      // await parent if child has it in dependency
      // if not, then is it marked as requiring a first exec
      // if yes add to watcher list.

      if (!account.offlineMode) {
        account.injectPlugin(ChainWorker, true);
      }

      if (account.type === WALLET_TYPES.HDWALLET) {
        // Ideally we should move out from worker to event based
        account.injectPlugin(BIP44Worker, true);
      }
      if (account.type === WALLET_TYPES.SINGLE_ADDRESS) {
        account.getAddress('0'); // We force what is usually done by the BIP44Worker.
      }
      if (!account.offlineMode) {
        account.injectPlugin(SyncWorker, true);
      }
    }

    _.each(userUnsafePlugins, (UnsafePlugin) => {
      account.injectPlugin(UnsafePlugin, account.allowSensitiveOperations);
    });

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
        self.events.emit(EVENTS.READY);
        self.isReady = true;
        clearInterval(self.readinessInterval);
        return res(true);
      }
    }, 600);

    self.events.emit(EVENTS.STARTED);
  });
}
module.exports = _initializeAccount;

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
    }

    _.each(userUnsafePlugins, (UnsafePlugin) => {
      account.injectPlugin(UnsafePlugin, account.allowSensitiveOperations);
    });


    const sendReady = () => {
      if (!self.isReady) {
        self.events.emit(EVENTS.READY);
        self.isReady = true;
      }
    };
    const recursivelyGenerateAddresses = async () => {
      const bip44worker = account.getWorker('BIP44Worker');
      const syncWorker = account.getWorker('syncWorker');
      const exec = async () => {
        await syncWorker.execute();
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
        // If both of the plugins are present
        // We need to tweak it a little bit to have BIP44 ensuring address
        // while SyncWorker fetch'em on network
        clearInterval(self.readinessInterval);
        if (account.hasPlugins([BIP44Worker, SyncWorker])) {
          recursivelyGenerateAddresses()
            .then(() => {
              sendReady();
              return res(true);
            })
            .catch(() => {
              console.error('Unable to generate addresses');
              sendReady();
              return res(true);
            });
        } else {
          sendReady();
          return res(true);
        }
      }
    }, 600);

    self.events.emit(EVENTS.STARTED);
  });
}

module.exports = _initializeAccount;

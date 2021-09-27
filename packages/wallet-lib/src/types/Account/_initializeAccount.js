const _ = require('lodash');
const logger = require('../../logger');
const TransactionSyncStreamWorker = require('../../plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker');
const ChainPlugin = require('../../plugins/Plugins/ChainPlugin');
const IdentitySyncWorker = require('../../plugins/Workers/IdentitySyncWorker');
const EVENTS = require('../../EVENTS');
const { WALLET_TYPES } = require('../../CONSTANTS');

const ensureAddressesToGapLimit = require('../../utils/bip44/ensureAddressesToGapLimit');

// eslint-disable-next-line no-underscore-dangle
async function _initializeAccount(account, userUnsafePlugins) {
  const self = account;
  // We run faster in offlineMode to speed up the process when less happens.
  const readinessIntervalTime = (account.offlineMode) ? 50 : 200;
  // TODO: perform rejection with a timeout
  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    try {
      if (account.injectDefaultPlugins) {
      // TODO: Should check in other accounts if a similar is setup already
      // TODO: We want to sort them by dependencies and deal with the await this way
      // await parent if child has it in dependency
      // if not, then is it marked as requiring a first exec
      // if yes add to watcher list.

        if ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(account.walletType)) {
          ensureAddressesToGapLimit(
            account.store.wallets[account.walletId],
            account.walletType,
            account.index,
            account.getAddress.bind(account),
          );
        } else {
          await account.getAddress('0'); // We force what is usually done by the BIP44Worker.
        }

        if (!account.offlineMode) {
          await account.injectPlugin(ChainPlugin, true);

          // Transaction sync worker
          await account.injectPlugin(TransactionSyncStreamWorker, true);

          if (account.walletType === WALLET_TYPES.HDWALLET) {
            await account.injectPlugin(IdentitySyncWorker, true);
          }
        }
      }

      _.each(userUnsafePlugins, (UnsafePlugin) => {
        account.injectPlugin(UnsafePlugin, account.allowSensitiveOperations);
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

          switch (account.walletType) {
            case WALLET_TYPES.PRIVATEKEY:
            case WALLET_TYPES.SINGLE_ADDRESS:
              account.generateAddress(0);
              sendReady();
              return resolve(true);
            case WALLET_TYPES.PUBLICKEY:
            case WALLET_TYPES.ADDRESS:
              account.generateAddress(0);
              sendReady();
              return resolve(true);
            default:
              break;
          }

          if (!account.injectDefaultPlugins) {
            sendReady();
            return resolve(true);
          }

          if ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(account.walletType)) {
            ensureAddressesToGapLimit(
              account.store.wallets[account.walletId],
              account.walletType,
              account.index,
              account.getAddress.bind(account),
            );
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

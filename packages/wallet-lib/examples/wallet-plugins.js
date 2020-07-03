const { Wallet } = require('../src');
const logger = require('../src/logger');
// This is a ColdStorage worker. It ran each X, verify a condition (execute function), and
const ColdStorageWorker = require('./workers/ColdStorageWorker');

// Wallet Consolidator is a standard plugin, when added it will offer new
// functionalities to the account. Such as 'consolidateWallet' method.
const WalletConsolidator = require('./stdPlugins/WalletConsolidator');

// This will be used by the coldStorageWorker which is responsible for performing cold-storage on
// this address.
const coldStorageAddress = 'yb67GKjkk4AMrJcqoedCjeemFGo9bDovNS';

const wallet = new Wallet({
  mode: 'light',
  injectDefaultPlugins: false, // Will not inject default plugins (BIP44, SyncWorker)
  // Will add these plugin instead, one is already init to show that both are fine to used.
  // The order has it's importance, here ColdStorageWorker will use WalletConsolidator as a depts.
  plugins: [WalletConsolidator, new ColdStorageWorker({ address: coldStorageAddress })],
});


wallet.getAccount({ index: 0 })
  .then((account) => {
    const showcasePlugin = async () => {
      const walletConsolidator = account.getPlugin('walletConsolidator');
      const consolidate = await walletConsolidator.consolidateWallet();

      const preparedTransaction = consolidate.prepareTransaction();

      logger.info('RawTx', preparedTransaction.toString());
      logger.info('Broadcast', await preparedTransaction.broadcast());
    };

    logger.info('Balance', account.getTotalBalance());
    logger.info('Funding address', account.getUnusedAddress().address);

    return showcasePlugin();
  });

const { Wallet } = require('../index');
// This is a ColdStorage worker. It ran each X, verify a condition (execute function), and
const ColdStorageWorker = require('./workers/ColdStorageWorker');

// This is a DPA showcasing a DOC notarization
const DPADoc = require('./DPAs/DPADoc');

// Wallet Consolidator is a standard plugin, when added it will offer new
// functionnalities to the account. Such as 'consolidateWallet' method.
const WalletConsolidator = require('./stdPlugins/WalletConsolidator');

// This will be used by the coldStorageWorker which is responsible for performing cold-storage on
// this address.
const coldStorageAddress = 'yb67GKjkk4AMrJcqoedCjeemFGo9bDovNS';

const wallet = new Wallet({
  mode: 'light',
  transport: 'insight',
  injectDefaultPlugins: false, // Will not inject default plugins (BIP44, SyncWorker)
  // Will add these plugin instead, one is already init to show that both are fine to used.
  // The order has it's importance, here ColdStorageWorker will use WalletConsolidator as a depts.
  plugins: [WalletConsolidator, DPADoc, new ColdStorageWorker({ address: coldStorageAddress })],
});

const account = wallet.getAccount(0);

const start = async () => {
  console.log('Balance', account.getBalance());
  console.log('Funding address', account.getUnusedAddress().address);

  // await showcasePlugin();
  // await showcaseDPA();
};


const showcasePlugin = async () => {
  const walletConsolidator = account.getPlugin('walletConsolidator');
  const consolidate = await walletConsolidator.consolidateWallet();

  const preparedTransaction = consolidate.prepareTransaction();
  console.log('RawTx', preparedTransaction.toString());
  // console.log('Broadcast', await preparedTransaction.broadcast());
};

const showcaseDPA = async () => {
  const dpaDoc = account.getDPA('DPADoc');
  const documentPath = `${__dirname}/document.txt`;
  const notarize = await dpaDoc.notarizeDocument(documentPath);

  console.log('Notarized ?', notarize);
};

account.events.on('ready', start);

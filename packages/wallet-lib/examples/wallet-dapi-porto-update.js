const DAPIClient = require('@dashevo/dapi-client');
const logger = require('../src/logger');
const { Wallet, EVENTS } = require('../src');

const transport = new DAPIClient({
  seeds: [{ service: '18.237.69.61:3000' }],
  timeout: 20000,
  retries: 5,
});

const wallet = new Wallet({
  // mnemonic: 'hurdle rebel rebuild skull add sustain local curious click industry illegal joy',
  mnemonic: 'stand reflect diagram busy sport clarify once ozone evoke doctor vessel aisle',
  network: 'testnet',
  transport,
});

const account = wallet.getAccount();
logger.info('Exported wallet', wallet.exportWallet());

const start = async () => {
  // const HDPubKey = account.keyChain.getKeyForPath('m/44\'/1\'/0\'/0', 'HDPublicKey');
  // console.log(HDPubKey.toString());

  // setTimeout(async ()=>{
  //   const receiveAddress = 'yZHMa5Xr6iEKtoei22wqtuaJGtxtSQZcAz';
  //   await account.broadcastTransaction(account.createTransaction({
  //     recipient: receiveAddress, amount: 0.2000
  //   }));
  //
  // },2000)

  logger.info('Total Balance', account.getTotalBalance());
  logger.info('Confirmed Balance', account.getConfirmedBalance());
  logger.info('Unconfirmed Balance', account.getUnconfirmedBalance());
  logger.info('UTXOS', account.getUTXOS());
  logger.info('Unused Address', account.getUnusedAddress());
};


account.events.on(EVENTS.PREFETCHED, () => {
  logger.info('PREFETCHED');
});
account.events.on(EVENTS.CREATED, () => {
  logger.info('CREATED');
});
account.events.on(EVENTS.STARTED, () => {
  logger.info('STARTED');
});
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, () => {
  logger.info('BALANCE_CHANGED');
});
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, () => {
  logger.info('UNCONFIRMED_BALANCE_CHANGED');
});
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => {
  logger.info('BLOCKHEIGHT_CHANGED', info);
});
account.events.on(EVENTS.BLOCK, () => {
  logger.info('BLOCK');
});
account.events.on(EVENTS.READY, () => {
  logger.info('READY');
  start();
});
account.events.on(EVENTS.FETCHED_ADDRESS, () => {
  logger.info('FETCHED_ADDRESS');
});
account.events.on(EVENTS.ERROR_UPDATE_ADDRESS, () => {
  logger.info('ERROR_UPDATE_ADDRESS');
});
account.events.on(EVENTS.FETCHED_TRANSACTIONS, () => {
  logger.info('FETCHED_TRANSACTIONS');
});
account.events.on(EVENTS.FETCHED_UNCONFIRMED_TRANSACTION, (info) => {
  logger.info('FETCHED_UNCONFIRMED_TRANSACTION', info);
});
account.events.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, (info) => {
  logger.info('FETCHED_CONFIRMED_TRANSACTION', info);
});
account.events.on(EVENTS.GENERATED_ADDRESS, () => {
  // logger.info('GENERATED_ADDRESS');
});
account.events.on(EVENTS.DISCOVERY_STARTED, () => {
  logger.info('DISCOVERY_STARTED');
});
account.events.on(EVENTS.CONFIGURED, () => {
  logger.info('CONFIGURED');
});
account.events.on(EVENTS.SAVE_STATE_FAILED, () => {
  logger.info('SAVE_STATE_FAILED');
});
account.events.on(EVENTS.SAVE_STATE_SUCCESS, () => {
  logger.info('SAVE_STATE_SUCCESS');
});
account.events.on(EVENTS.REHYDRATE_STATE_FAILED, () => {
  logger.info('REHYDRATE_STATE_FAILED');
});
account.events.on(EVENTS.REHYDRATE_STATE_SUCCESS, () => {
  logger.info('REHYDRATE_STATE_SUCCESS');
});

const DAPIClient = require('@dashevo/dapi-client');
const { Wallet, EVENTS } = require('../src');

const transport = new DAPIClient({
  seeds: [{ service: '18.237.69.61:3000' }],
  timeout: 20000,
  retries: 5,
});

const wallet = new Wallet({
  mnemonic: 'hurdle rebel rebuild skull add sustain local curious click industry illegal joy',
  network: 'testnet',
  transport,
});

const account = wallet.getAccount();
console.log(wallet.exportWallet());

const start = async () => {
  // const HDExtPubKey = account.keyChain.getKeyForPath('m/44\'/1\'/0\'/0', 'HDPublicKey');
  // console.log(HDExtPubKey.toString());

  console.log(account.getBalance());
  console.log(account.getUTXOS());
  console.log(account.getUnusedAddress());
};


account.events.on(EVENTS.PREFETCHED, (info) => {
  console.log('PREFETCHED');
});
account.events.on(EVENTS.CREATED, (info) => {
  console.log('CREATED');
});
account.events.on(EVENTS.STARTED, (info) => {
  console.log('STARTED');
});
account.events.on(EVENTS.BALANCE_CHANGED, (info) => {
  console.log('BALANCE_CHANGED');
});
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => {
  console.log('UNCONFIRMED_BALANCE_CHANGED');
});
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => {
  console.log('BLOCKHEIGHT_CHANGED');
});
account.events.on(EVENTS.BLOCK, (info) => {
  console.log('BLOCK');
});
account.events.on(EVENTS.READY, (info) => {
  console.log('READY');
  start();
});
account.events.on(EVENTS.FETCHED_ADDRESS, (info) => {
  console.log('FETCHED_ADDRESS');
});
account.events.on(EVENTS.ERROR_UPDATE_ADDRESS, (info) => {
  console.log('ERROR_UPDATE_ADDRESS');
});
account.events.on(EVENTS.FETCHED_TRANSACTIONS, (info) => {
  console.log('FETCHED_TRANSACTIONS');
});
account.events.on(EVENTS.FETCHED_UNCONFIRMED_TRANSACTION, (info) => {
  console.log('FETCHED_UNCONFIRMED_TRANSACTION');
});
account.events.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, (info) => {
  console.log('FETCHED_CONFIRMED_TRANSACTION');
});
account.events.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, (info) => {
  console.log('FETCHED_CONFIRMED_TRANSACTION');
});
account.events.on(EVENTS.GENERATED_ADDRESS, (info) => {
  console.log('GENERATED_ADDRESS');
});
account.events.on(EVENTS.DISCOVERY_STARTED, (info) => {
  console.log('DISCOVERY_STARTED');
});
account.events.on(EVENTS.CONFIGURED, (info) => {
  console.log('CONFIGURED');
});
account.events.on(EVENTS.SAVE_STATE_FAILED, (info) => {
  console.log('SAVE_STATE_FAILED');
});
account.events.on(EVENTS.SAVE_STATE_SUCCESS, (info) => {
  console.log('SAVE_STATE_SUCCESS');
});
account.events.on(EVENTS.REHYDRATE_STATE_FAILED, (info) => {
  console.log('REHYDRATE_STATE_FAILED');
});
account.events.on(EVENTS.REHYDRATE_STATE_SUCCESS, (info) => {
  console.log('REHYDRATE_STATE_SUCCESS');
});

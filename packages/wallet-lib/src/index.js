const Dashcore = require('@dashevo/dashcore-lib');
const { EventEmitter } = require('events');
const KeyChain = require('./KeyChain');

const { Registration, TopUp } = Dashcore.Transaction.SubscriptionTransactions;
const { Transaction, PrivateKey } = Dashcore;

const EVENT_CONSTANTS = {
  SYNCED: 'SYNCED',
};

/**
 * Returns a new wallet object.
 * @param {DAPIClient} DAPIClient
 * @param {string} [privateHDKey]
 * @return {Wallet} - new wallet object
 */
const createWallet = (DAPIClient, privateHDKey) => ({
  DAPIClient,
  events: new EventEmitter(),
  privateHDKey,
  synced: false,
});

/**
 * Returns a new synchronized wallet object.
 * @param {Wallet} wallet
 * @return {Wallet} - newly synchronized wallet object
 */
const syncWallet = (wallet) => {
  wallet.events.emit(EVENT_CONSTANTS.SYNCED);
  return Object.assign(wallet, { synced: true });
};

// TODO: Do we need this, or can we export KeyChain as a separate bundled library?
const getPrivateKeyForSigning = wallet => KeyChain.getNewPrivateKey(wallet.privateHDKey);

/**
 * @param {Wallet} wallet
 * @return {Array<object>} - list of unspent outputs for the wallet
 */
const getUTXO = async (wallet) => { throw new Error('Not Implemented'); };

/**
 * @param {Wallet} wallet
 * @return {string} - new change address
 */
const getNewAddress = (wallet, derivationPath) => {
  const newKey = KeyChain.getNewPrivateKey(wallet.privateHDKey, derivationPath);
  return String(newKey.toAddress());
};

/**
 * Broadcasts transaction to the network.
 * @param {Wallet} wallet
 * @param {string} rawTransaction
 */
const sendTransaction = (wallet, rawTransaction) => wallet.DAPIClient
  .sendRawTransaction(rawTransaction);

/**
 * Signs transaction.
 * @param {Wallet} wallet
 * @param {string} rawTx - hex string representing transaction to sign
 * @return {string} - hex string representing signed transaction
 */
const signTransaction = (wallet, rawTransaction) => {
  const privateKeyForSigning = getPrivateKeyForSigning(wallet);
  const tx = new Transaction(rawTransaction);
  tx.sign(privateKeyForSigning);
  return tx.serialize();
};

/*
 * Evo L1 stuff
 */

/**
 * @param {Wallet} wallet
 * @param {string} username
 * @returns {string} - hex string containing user registration
 */
const createRegistration = (wallet, username) => {
  const privateKey = new PrivateKey(wallet.keychain.getNewPrivateKey());
  return Registration.createRegistration(username, privateKey).serialize();
};

/**
 * @param {Wallet} wallet
 * @param {string} rawRegistration - hex string representing user registration data
 * @param {number} [funding] - default funding for the account in duffs. Optional. If left empty,
 * funding will be 0.
 * @return {string} - user id
 */
const registerUser = async (wallet, rawRegistration, funding = 0) => {
  const regTx = new Registration(rawRegistration);
  const UTXO = await this.getUTXO();
  const newAddress = getNewAddress(wallet);
  regTx.fund(UTXO, newAddress, funding);
  const serializedRegTx = regTx.serialize();
  const signedTx = signTransaction(wallet, serializedRegTx);
  return this.sendTransaction(signedTx);
};

/**
 * @param {Wallet} wallet
 * @param {string} userId
 * @param {number} amount - top up amount in duffs
 * @return {Promise<string>} - tx id
 */
const topUpUserCredits = async (wallet, userId, amount) => {
  const inputs = await getUTXO(wallet);
  const subTx = new TopUp();
  const newAddress = getNewAddress(wallet);
  subTx.fund(userId, amount, inputs, newAddress);
  const signedTx = this.signTransaction(subTx.serialize());
  return this.sendTransaction(signedTx);
};

/**
 * @param {Wallet} wallet
 * @param rawHeader
 * @returns {Promise<string>}
 */
const signStateTransitionHeader = async (wallet, rawHeader) => {
  const privateKeyForSigning = getPrivateKeyForSigning(wallet);
  const ts = new Transaction(rawHeader);
  ts.sign(privateKeyForSigning);
  return ts.serialize();
};

module.exports = {
  createWallet,
  syncWallet,
  getPrivateKeyForSigning,
  getUTXO,
  getNewAddress,
  sendTransaction,
  signTransaction,
  createRegistration,
  registerUser,
  topUpUserCredits,
  signStateTransitionHeader,
};

const DashcoreLib = require('@dashevo/dashcore-lib');
const { EventEmitter } = require('events');
const KeyChain = require('./KeyChain');

const { Registration, TopUp } = DashcoreLib.Transaction.SubscriptionTransactions;
const { Transaction, PrivateKey } = DashcoreLib;

class Wallet {
  /**
   *
   * @param {DAPIClient} DAPIClient
   * @param {string} [privateHDKey]
   */
  constructor(DAPIClient, privateHDKey) {
    this.keychain = new KeyChain(privateHDKey);
    this.DAPIClient = DAPIClient;
    this.synced = false;
    this.events = new EventEmitter();
    this.eventConstants = {
      synced: 'synced',
    };

    this.synchronize();
  }

  getPrivateKeyForSigning() {
    // Mock up
    return this.keychain.getNewPrivateKey();
  }

  synchronize() {
    // Just mock up
    this.synced = true;
    this.events.emit(this.eventConstants.synced);
  }

  /**
   * @return {Array<object>} - list of unspent outputs for the wallet
   */
  async getUTXO() {
    throw new Error('Not Implemented');
  }

  /**
   * @return {string} - new change address
   */
  getNewAddress() {
    const newKey = this.keychain.getNewPrivateKey();
    return newKey.toAddress().toString();
  }

  /**
   * Broadcasts transaction to the network
   * @param {string} rawTransaction
   */
  sendTransaction(rawTransaction) {
    return this.DAPIClient.sendRawTransaction(rawTransaction);
  }

  /**
   * Signs transaction
   * @param {string} rawTx - hex string representing transaction to sign
   * @return {string} - hex string representing signed transaction
   */
  signTransaction(rawTx) {
    const tx = new Transaction(rawTx);
    tx.sign(this.getPrivateKeyForSigning());
    return tx.serialize();
  }

  /*
   * Evo L1 stuff
   */

  /**
   * @param {string} username
   * @returns {string} - hex string containing user registration
   */
  createRegistration(username) {
    const privateKey = new PrivateKey(this.keychain.getNewPrivateKey());
    return Registration.createRegistration(username, privateKey).serialize();
  }

  /**
   * @param {string} rawRegistration - hex string representing user registration data
   * @param {number} [funding] - default funding for the account in duffs. Optional. If left empty,
   * funding will be 0.
   * @return {string} - user id
   */
  async registerUser(rawRegistration, funding = 0) {
    const regTx = new Registration(rawRegistration);
    const UTXO = await this.getUTXO();
    regTx.fund(UTXO, this.getNewAddress(), funding);
    const signedTx = this.signTransaction(regTx.serialize());
    return this.sendTransaction(signedTx);
  }

  /**
   * @param {string} userId
   * @param {number} amount - top up amount in duffs
   * @return {Promise<string>} - tx id
   */
  async topUpUserCredits(userId, amount) {
    const inputs = await this.getUTXO();
    const subTx = new TopUp();
    subTx.fund(userId, amount, inputs, this.getNewAddress());
    const signedTx = this.signTransaction(subTx.serialize());
    return this.sendTransaction(signedTx);
  }

  async signStateTransitionHeader(rawHeader) {
    const ts = new Transaction(rawHeader);
    ts.sign(this.getPrivateKeyForSigning());
    return ts.serialize();
  }
}

module.exports = Wallet;

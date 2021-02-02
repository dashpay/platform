const EventEmitter = require('events');
const TransactionWaitPeriodExceededError = require('../../errors/TransactionWaitPeriodExceededError');

const TX_QUERY = 'tm.event = \'Tx\'';
const NEW_BLOCK_QUERY = 'tm.event = \'NewBlock\'';
const events = {
  NEW_BLOCK: 'block',
};

class BlockchainListener extends EventEmitter {
  /**
   * @param {WsClient} tenderdashWsClient
   */
  constructor(tenderdashWsClient) {
    super();
    this.wsClient = tenderdashWsClient;
  }

  /**
   * Returns an event name for a specific hash
   * @private
   * @param {string} transactionHashString
   * @return {string}
   */
  static getTransactionEventName(transactionHashString) {
    return `transaction:${transactionHashString}`;
  }

  /**
   * Subscribe to transactions and attach transaction event handler
   */
  start() {
    this.wsClient.subscribe(TX_QUERY);
    this.wsClient.on(TX_QUERY, this.emitTransaction.bind(this));

    this.wsClient.subscribe(NEW_BLOCK_QUERY);
    this.wsClient.on(NEW_BLOCK_QUERY, (message) => {
      this.emit(events.NEW_BLOCK, message);
    });
  }

  /**
   * Creates promisified event handler
   * @private
   * @param {string} eventName
   * @param {function} resolve
   * @return {function}
   */
  createPromiseHandler(eventName, resolve) {
    const handler = (data) => {
      this.off(eventName, handler);
      resolve(data);
    };

    return handler;
  }

  /**
   * Emits transaction:%tx_hash% if there's a transaction in the message
   * @private
   * @param {Object} message
   */
  emitTransaction(message) {
    const hashArray = message && message.events ? message.events['tx.hash'] : null;
    const hashString = Array.isArray(hashArray) && hashArray[0];
    if (!hashString) {
      return;
    }

    this.emit(BlockchainListener.getTransactionEventName(hashString), message);
  }

  /**
   * Returns data for a transaction or rejects after a timeout
   * @param {string} hashString - transaction hash to resolve data for
   * @param {number} [timeout] - timeout to reject after
   * @return {Promise<Object>}
   */
  waitForTransaction(hashString, timeout = 60000) {
    const topic = BlockchainListener.getTransactionEventName(hashString);
    let handler;

    return Promise.race([
      new Promise((resolve) => {
        handler = this.createPromiseHandler(topic, resolve);
        this.on(topic, handler);
      }),
      new Promise((resolve, reject) => {
        setTimeout(() => {
          this.off(topic, handler);
          reject(new TransactionWaitPeriodExceededError(hashString));
        }, timeout);
      }),
    ]);
  }

  waitForNextBlock() {
    return new Promise((resolve) => {
      this.once(events.NEW_BLOCK, resolve);
    });
  }

  async waitForBlocks(countToWait = 1) {
    let blocksSeen = 0;
    while (blocksSeen !== countToWait) {
      await this.waitForNextBlock();
      blocksSeen += 1;
    }
  }
}

BlockchainListener.TX_QUERY = TX_QUERY;
BlockchainListener.NEW_BLOCK_QUERY = NEW_BLOCK_QUERY;
BlockchainListener.events = events;

module.exports = BlockchainListener;

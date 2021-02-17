const crypto = require('crypto');

const EventEmitter = require('events');

const TX_QUERY = 'tm.event = \'Tx\'';
const NEW_BLOCK_QUERY = 'tm.event = \'NewBlock\'';
const EVENTS = {
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
   *
   * @param {string} transactionHashString
   * @return {string}
   */
  static getTransactionEventName(transactionHashString) {
    return `transaction:${transactionHashString}`;
  }

  /**
   *
   * @param transactionHashString
   * @return {string}
   */
  static getTransactionAddedToTheBlockEventName(transactionHashString) {
    return `blockTransactionAdded:${transactionHashString}`;
  }

  /**
   * Subscribe to blocks and transaction results
   */
  start() {
    // Emit transaction results
    this.wsClient.subscribe(TX_QUERY);
    this.wsClient.on(TX_QUERY, (message) => {
      const hashArray = message && message.events ? message.events['tx.hash'] : null;
      const hashString = Array.isArray(hashArray) && hashArray[0];
      if (!hashString) {
        return;
      }

      this.emit(BlockchainListener.getTransactionEventName(hashString), message);
    });

    // Emit blocks and contained transactions
    this.wsClient.subscribe(NEW_BLOCK_QUERY);
    this.wsClient.on(NEW_BLOCK_QUERY, (message) => {
      this.emit(EVENTS.NEW_BLOCK, message);

      // Emit transaction hashes from block
      message.data.value.block.data.txs.forEach((base64tx) => {
        const transaction = Buffer.from(base64tx, 'base64');
        const hashString = crypto.createHash('sha256')
          .update(transaction)
          .digest()
          .toString('hex');

        this.emit(
          BlockchainListener.getTransactionAddedToTheBlockEventName(hashString),
          transaction,
        );
      });
    });
  }
}

BlockchainListener.TX_QUERY = TX_QUERY;
BlockchainListener.NEW_BLOCK_QUERY = NEW_BLOCK_QUERY;
BlockchainListener.EVENTS = EVENTS;

module.exports = BlockchainListener;

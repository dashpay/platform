const EventEmitter = require('events');
const logger = require('../../logger');

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
   * Subscribe to blocks and transaction results
   */
  start() {
    const processLogger = logger.child({
      process: 'BlockchainListener',
    });

    processLogger.info('Subscribed to state transition results');

    // Emit transaction results
    this.wsClient.subscribe(TX_QUERY);
    this.wsClient.on(TX_QUERY, (message) => {
      const [hashString] = (message.events || []).map((event) => {
        const hashAttribute = event.attributes.find((attribute) => attribute.key === 'hash');

        if (!hashAttribute) {
          return null;
        }

        return hashAttribute.value;
      }).filter((hash) => hash !== null);

      if (!hashString) {
        return;
      }

      processLogger.trace(`received transaction result for ${hashString}`);

      this.emit(BlockchainListener.getTransactionEventName(hashString), message);
    });

    // TODO: It's not using
    // Emit blocks and contained transactions
    // this.wsClient.subscribe(NEW_BLOCK_QUERY);
    // this.wsClient.on(NEW_BLOCK_QUERY, (message) => this.emit(EVENTS.NEW_BLOCK, message));
  }
}

BlockchainListener.TX_QUERY = TX_QUERY;
BlockchainListener.NEW_BLOCK_QUERY = NEW_BLOCK_QUERY;
BlockchainListener.EVENTS = EVENTS;

module.exports = BlockchainListener;

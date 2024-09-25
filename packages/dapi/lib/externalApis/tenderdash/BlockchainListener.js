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

    this.processLogger = logger.child({
      process: 'BlockchainListener',
    });
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
    // Emit transaction results
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

      this.processLogger.trace(`Received transaction result for ${hashString}`);

      this.emit(BlockchainListener.getTransactionEventName(hashString), message);
    });

    // Emit blocks and contained transactions
    this.wsClient.on(NEW_BLOCK_QUERY, (message) => {
      this.processLogger.trace('Received new platform block');

      this.emit(EVENTS.NEW_BLOCK, message)
    });

    this.wsClient.on('connect', () => {
      this.#subscribe();
    });

    if (this.wsClient.isConnected) {
      this.#subscribe();
    }
  }

  #subscribe() {
    this.wsClient.subscribe(NEW_BLOCK_QUERY);
    this.wsClient.subscribe(TX_QUERY);
    this.processLogger.debug('Subscribed to platform blockchain events');
  }
}

BlockchainListener.TX_QUERY = TX_QUERY;
BlockchainListener.NEW_BLOCK_QUERY = NEW_BLOCK_QUERY;
BlockchainListener.EVENTS = EVENTS;

module.exports = BlockchainListener;

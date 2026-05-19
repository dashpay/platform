import EventEmitter from 'events';

import EVENTS from '../EVENTS.js';
import logger from '../logger/index.js';

/**
 * @abstract
 */
class AbstractTransport extends EventEmitter {
  constructor() {
    super();

    this.state = {
      block: null,
      blockHeaders: null,
      // Executors are Interval
      executors: {
        blocks: null,
        blockHeaders: null,
        addresses: null,
      },
      addressesTransactionsMap: {},
      subscriptions: {
        addresses: {},
      },
    };
  }

  announce(eventName, args) {
    logger.silly(`Transporter.announce(${eventName})`);
    switch (eventName) {
      case EVENTS.BLOCKHEADER:
      case EVENTS.BLOCKHEIGHT_CHANGED:
      case EVENTS.BLOCK:
      case EVENTS.TRANSACTION:
      case EVENTS.FETCHED_TRANSACTION:
      case EVENTS.FETCHED_ADDRESS:
        this.emit(eventName, { type: eventName, payload: args });
        break;
      default:
        this.emit(eventName, { type: eventName, payload: args });
        logger.warn('Transporter - Not implemented, announce of ', eventName, args);
    }
  }

  disconnect() {
    const { executors, subscriptions } = this.state;

    clearInterval(subscriptions.blocks);
    clearInterval(subscriptions.blockHeaders);

    // eslint-disable-next-line guard-for-in,no-restricted-syntax
    for (const addr in subscriptions.addresses) {
      clearInterval(addr);
      delete this.state.subscriptions.addresses[addr];
    }

    clearInterval(executors.blocks);
    clearInterval(executors.blockHeaders);
    clearInterval(executors.addresses);
  }
}

export default AbstractTransport;
const EVENTS = require('../../../../EVENTS');
const logger = require('../../../../logger');

module.exports = function announce(eventName, args) {
  logger.silly(`Transporter.announce(${eventName})`);
  switch (eventName) {
    case EVENTS.BLOCKHEADER:
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
};

const EVENTS = require('../../../EVENTS');
const logger = require('../../../logger');

module.exports = function announce(type, el) {
  logger.silly(`SyncWorker.announce(${type})`);
  switch (type) {
    case EVENTS.BLOCK:
    case EVENTS.FETCHED_ADDRESS:
    case EVENTS.BLOCKHEIGHT_CHANGED:
      this.parentEvents.emit(type, { type, payload: el });
      break;
    default:
      this.parentEvents.emit(type, { type, payload: el });
      logger.warn('SyncWorker - Not implemented, announce of ', { type, el });
  }
};

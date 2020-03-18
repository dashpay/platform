const EVENTS = require('../../../EVENTS');
const logger = require('../../../logger');

module.exports = function announce(type, el) {
  logger.silly(`SyncWorker.announce(${type})`);
  switch (type) {
    case EVENTS.BLOCK:
      this.parentEvents.emit(EVENTS.BLOCK, { type: EVENTS.BLOCK, payload: el });
      break;
    case EVENTS.FETCHED_ADDRESS:
      this.parentEvents.emit(EVENTS.FETCHED_ADDRESS, { type: EVENTS.FETCHED_ADDRESS, payload: el });
      break;
    case EVENTS.BLOCKHEIGHT_CHANGED:
      this.parentEvents.emit(EVENTS.BLOCKHEIGHT_CHANGED,
        { type: EVENTS.BLOCKHEIGHT_CHANGED, payload: el });
      break;
    default:
      this.parentEvents.emit(type, { type, paylaod: el });
      logger.warn('SyncWorker - Not implemented, announce of ', { type, el });
  }
};

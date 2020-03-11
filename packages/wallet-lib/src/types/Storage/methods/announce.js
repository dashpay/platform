const logger = require('../../../logger');
const EVENTS = require('../../../EVENTS');

/**
 * Used to announce some events.
 * @param type
 * @param el
 * @return {boolean}
 */
const announce = function (type, el) {
  switch (type) {
    case EVENTS.BLOCK:
    case EVENTS.BLOCKHEADER:
    case EVENTS.BLOCKHEIGHT_CHANGED:
    case EVENTS.CONFIRMED_BALANCE_CHANGED:
    case EVENTS.UNCONFIRMED_BALANCE_CHANGED:
    case EVENTS.FETCHED_UNCONFIRMED_TRANSACTION:
    case EVENTS.FETCHED_CONFIRMED_TRANSACTION:
      this.emit(type, { type, payload: el });
      break;
    default:
      this.emit(type, { type, payload: el });
      logger.warn('Storage - Not implemented, announce of ', type, el);
  }
  return true;
};
module.exports = announce;

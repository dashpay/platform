const ChainStore = require('../../ChainStore/ChainStore');
const EVENTS = require('../../../EVENTS');

const EVENTS_TO_FORWARD = [
  EVENTS.FETCHED_CONFIRMED_TRANSACTION,
];

/**
 * Create when does not yet exist a chainStore
 * @param network
 * @return {boolean}
 */
const createChainStore = function createChain(network) {
  if (!this.chains.has(network.toString())) {
    const chainStore = new ChainStore(network.toString());
    this.chains.set(network.toString(), chainStore);

    EVENTS_TO_FORWARD.forEach((event) => {
      chainStore.on(event, (data) => {
        this.emit(event, { type: event, payload: data });
      });
    });

    return true;
  }
  return false;
};
module.exports = createChainStore;

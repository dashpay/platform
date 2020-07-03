const EVENTS = require('../../../EVENTS');

module.exports = function setupListeners() {
  const { storage, transport } = this;

  // For each new transaction emitted by transport, we import to storage
  // It will also look-up for UTXO
  transport.on(EVENTS.FETCHED_TRANSACTION, async (ev) => {
    const { payload: transaction } = ev;
    // Storage.importTransaction will announce the TX to parent
    await storage.importTransaction(transaction);
  });

  // The same is being done for fetch_address, but we also announce it.
  transport.on(EVENTS.FETCHED_ADDRESS, async (ev) => {
    const { payload: address } = ev;
    this.announce(EVENTS.FETCHED_ADDRESS, address);
  });
};

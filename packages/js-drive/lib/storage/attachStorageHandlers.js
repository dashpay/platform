const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 * Remove State Transition Packet from blockchain when wrong sequence.
 * Remove all State Transition Packets from blockchain when reset.
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {STPacketIpfsRepository} stPacketRepository
 */
function attachStorageHandlers(
  readerMediator,
  stPacketRepository,
) {
  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION, async ({ stateTransition }) => {
    const packetCid = stateTransition.getPacketCID();
    await stPacketRepository.download(packetCid);
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_STALE, async ({ stateTransition }) => {
    const packetCid = stateTransition.getPacketCID();
    await stPacketRepository.delete(packetCid);
  });

  readerMediator.on(ReaderMediator.EVENTS.RESET, async () => {
    await stPacketRepository.deleteAll();
  });
}

module.exports = attachStorageHandlers;

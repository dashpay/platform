const ReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');

const PinPacketTimeoutError = require('./errors/PinPacketTimeoutError');

const rejectAfter = require('../util/rejectAfter');

/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 * Remove State Transition Packet from blockchain when wrong sequence.
 * Remove all State Transition Packets from blockchain when reset.
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {IpfsAPI} ipfsAPI
 * @param {RpcClient} rpcClient
 * @param {unpinAllIpfsPackets} unpinAllIpfsPackets
 * @param {number} ipfsPinTimeout
 */
function attachStorageHandlers(
  readerMediator,
  ipfsAPI,
  rpcClient,
  unpinAllIpfsPackets,
  ipfsPinTimeout,
) {
  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION, async ({ stateTransition }) => {
    const packetPath = stateTransition.getPacketCID().toBaseEncodedString();

    const pinPromise = ipfsAPI.pin.add(packetPath, { recursive: true });
    const error = new PinPacketTimeoutError();

    await rejectAfter(pinPromise, error, ipfsPinTimeout);
  });

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_STALE, async ({ stateTransition }) => {
    const packetPath = stateTransition.getPacketCID().toBaseEncodedString();
    await ipfsAPI.pin.rm(packetPath, { recursive: true });
  });

  readerMediator.on(ReaderMediator.EVENTS.RESET, async () => {
    await unpinAllIpfsPackets();
  });
}

module.exports = attachStorageHandlers;

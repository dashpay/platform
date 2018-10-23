const STHeadersReader = require('../blockchain/reader/STHeadersReader');
const ArrayBlockIterator = require('../blockchain/iterator/ArrayBlockIterator');
const StateTransitionHeaderIterator = require('../blockchain/iterator/StateTransitionHeaderIterator');

const PinPacketTimeoutError = require('./errors/PinPacketTimeoutError');

const rejectAfter = require('../util/rejectAfter');

/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 * Remove State Transition Packet from blockchain when wrong sequence.
 * Remove all State Transition Packets from blockchain when reset.
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {IpfsAPI} ipfsAPI
 * @param {unpinAllIpfsPackets} unpinAllIpfsPackets
 * @param {number} ipfsPinTimeout
 */
function attachStorageHandlers(stHeadersReader, ipfsAPI, unpinAllIpfsPackets, ipfsPinTimeout) {
  const { stHeaderIterator: { rpcClient } } = stHeadersReader;

  stHeadersReader.on(STHeadersReader.EVENTS.HEADER, async ({ header, block }) => {
    const packetPath = header.getPacketCID().toBaseEncodedString();

    const pinPromise = ipfsAPI.pin.add(packetPath, { recursive: true });
    const error = new PinPacketTimeoutError();

    try {
      await rejectAfter(pinPromise, error, ipfsPinTimeout);
    } catch (e) {
      const errorContext = {
        blockHeight: block.height,
        blockHash: block.hash,
        packetHash: header.extraPayload.hashSTPacket,
        packetCid: packetPath,
      };
      console.error(new Date(), e, errorContext);
    }
  });

  stHeadersReader.on(STHeadersReader.EVENTS.STALE_BLOCK, async (block) => {
    const blockIterator = new ArrayBlockIterator([block]);
    const stHeadersIterator = new StateTransitionHeaderIterator(blockIterator, rpcClient);

    let done;
    let header;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: header } = await stHeadersIterator.next()) {
      if (done) {
        break;
      }

      const packetPath = header.getPacketCID().toBaseEncodedString();
      await ipfsAPI.pin.rm(packetPath, { recursive: true });
    }
  });

  stHeadersReader.on(STHeadersReader.EVENTS.RESET, async () => {
    await unpinAllIpfsPackets();
  });
}

module.exports = attachStorageHandlers;

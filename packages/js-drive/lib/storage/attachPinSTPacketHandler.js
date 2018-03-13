const ArrayBlockIterator = require('../blockchain/ArrayBlockIterator');
const StateTransitionHeaderIterator = require('../blockchain/StateTransitionHeaderIterator');

/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {IpfsAPI} ipfsAPI
 */
module.exports = function attachPinSTPacketHandler(stHeadersReader, ipfsAPI) {
  const { stHeaderIterator: { rpcClient } } = stHeadersReader;

  stHeadersReader.on('header', (header) => {
    ipfsAPI.pin.add(header.getStorageHash(), { recursive: true });
  });

  stHeadersReader.on('wrongSequence', async (block) => {
    const blockIterator = new ArrayBlockIterator([block]);
    const stHeadersIterator = new StateTransitionHeaderIterator(blockIterator, rpcClient);

    let done;
    let header;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: header } = await stHeadersIterator.next()) {
      if (done) {
        break;
      }

      await ipfsAPI.pin.rm(header.getStorageHash(), { recursive: true });
    }
  });
};

const WrongBlocksSequenceError = require('../../lib/blockchain/WrongBlocksSequenceError');

/**
 * Add State Transition Packets from blockchain since block height
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {StateTransitionHeaderIterator} stHeaderIterator
 */
module.exports = async function pinSTPacketsSinceBlock(ipfsAPI, stHeaderIterator) {
  for (; ;) {
    let done;
    let header;

    try {
      ({ done, value: header } = await stHeaderIterator.next());
    } catch (e) {
      if (!(e instanceof WrongBlocksSequenceError)) {
        throw e;
      }

      stHeaderIterator.reset(true);

      let previousBlockHeight = stHeaderIterator.blockIterator.getBlockHeight() - 1;
      if (previousBlockHeight < 1) {
        previousBlockHeight = 1;
      }

      stHeaderIterator.blockIterator.setBlockHeight(previousBlockHeight);

      // TODO: Unpin ST packets which added since stableBlockHeight

      // eslint-disable-next-line no-continue
      continue;
    }

    if (done) {
      break;
    }

    // TODO: Check number of confirmations. Should be more or equal than 6?
    // TODO: Validate packet using header?

    await ipfsAPI.pin.add(header.getStorageHash(), { recursive: true });
  }
};

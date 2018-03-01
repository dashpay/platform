/**
 * Add State Transition Packets from blockchain since block height
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {StateTransitionHeaderIterator} stHeaderIterator
 */
module.exports = async function pinSTPacketsSinceBlock(ipfsAPI, stHeaderIterator) {
  let done;
  let stateTransitionHeader;

  // eslint-disable-next-line no-cond-assign
  while ({ done, value: stateTransitionHeader } = await stHeaderIterator.next()) {
    if (done) {
      break;
    }

    // TODO: Check number of confirmations. Should be more or equal than 6?
    // TODO: Validate packet using header?

    await ipfsAPI.pin.add(stateTransitionHeader.getStorageHash(), { recursive: true });
  }
};

/* eslint-disable no-await-in-loop,no-cond-assign */
/**
 * Add State Transitions from blockchain
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {StateTransitionHeaderIterator} stateTransitionHeaderIterator
 */
async function addStateTransitionsFromBlockchain(ipfsAPI, stateTransitionHeaderIterator) {
  let done;
  let stateTransitionHeader;

  while ({ done, value: stateTransitionHeader } = await stateTransitionHeaderIterator.next()) {
    if (done) {
      break;
    }

    // TODO: Check number of confirmations. Should be more or equal than 6?
    // TODO: Validate packet using header?

    await ipfsAPI.pin.add(stateTransitionHeader.storageHash, { recursive: true });
  }
}

module.exports = addStateTransitionsFromBlockchain;

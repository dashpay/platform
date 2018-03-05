/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {STHeadersIterationEventEmitter} iterationEmitter
 */
module.exports = function attachPinSTPacketHandler(ipfsAPI, iterationEmitter) {
  // TODO: Check number of confirmations. Should be more or equal than 6?

  iterationEmitter.on('header', (header) => {
    ipfsAPI.pin.add(header.getStorageHash(), { recursive: true });
  });
};

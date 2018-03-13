/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {IpfsAPI} ipfsAPI
 */
module.exports = function attachPinSTPacketHandler(stHeadersReader, ipfsAPI) {
  // TODO: Check number of confirmations. Should be more or equal than 6?

  stHeadersReader.on('header', (header) => {
    ipfsAPI.pin.add(header.getStorageHash(), { recursive: true });
  });
};

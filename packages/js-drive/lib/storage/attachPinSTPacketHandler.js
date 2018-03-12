/**
 * Add State Transition Packet from blockchain when new ST header will appear.
 *
 * @param {IpfsAPI} ipfsAPI
 * @param {STHeadersReader} stHeadersReader
 */
module.exports = function attachPinSTPacketHandler(ipfsAPI, stHeadersReader) {
  // TODO: Check number of confirmations. Should be more or equal than 6?

  stHeadersReader.on('header', (header) => {
    ipfsAPI.pin.add(header.getStorageHash(), { recursive: true });
  });
};

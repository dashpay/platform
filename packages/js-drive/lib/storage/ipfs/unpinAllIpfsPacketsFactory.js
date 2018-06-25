/**
 * @param ipfsApi
 * @returns {unpinAllIpfsPackets}
 */
function unpinAllPacketsFactory(ipfsApi) {
  /**
   * Unpin all IPFS recursive packets
   *
   * @typedef {Promise} unpinAllIpfsPackets
   * @returns {Promise<void>}
   */
  async function unpinAllIpfsPackets() {
    const pinset = await ipfsApi.pin.ls();
    const byPinType = type => pin => pin.type === type;
    const pins = pinset.filter(byPinType('recursive'));

    for (let index = 0; index < pins.length; index++) {
      const pin = pins[index];
      await ipfsApi.pin.rm(pin.hash);
    }
  }

  return unpinAllIpfsPackets;
}

module.exports = unpinAllPacketsFactory;

/**
 * @param {IpfsAPI} ipfsApi
 * @return {addSTPacket}
 */
module.exports = function addSTPacketFactory(ipfsApi) {
  /**
   * Store State Transition packet in IPFS
   *
   * Stores and pins ST packet to IPFS storage and returns its hash
   *
   * @typedef addSTPacket
   * @param {StateTransitionPacket} packet State Transition packet
   * @return {Promise<CID>}
   */
  async function addSTPacket(packet) {
    const packetData = packet.toJSON({ skipMeta: true });

    return ipfsApi.dag.put(packetData, { cid: packet.getCID() });
  }

  return addSTPacket;
};

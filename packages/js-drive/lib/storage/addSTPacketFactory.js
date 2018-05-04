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
   * @return {string}
   */
  async function addSTPacket(packet) {
    const cid = await ipfsApi.dag.put(packet, { format: 'dag-cbor', hashAlg: 'sha2-256' });
    return cid.toBaseEncodedString();
  }

  return addSTPacket;
};

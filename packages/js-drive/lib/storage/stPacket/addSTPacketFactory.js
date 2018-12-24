/**
 * @param {StateTransitionPacketIpfsRepository} stPacketRepository
 * @return {addSTPacket}
 */
module.exports = function addSTPacketFactory(stPacketRepository) {
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
    return stPacketRepository.store(packet);
  }

  return addSTPacket;
};

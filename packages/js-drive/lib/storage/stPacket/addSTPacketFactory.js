/**
 * @param {STPacketIpfsRepository} stPacketRepository
 * @return {addSTPacket}
 */
module.exports = function addSTPacketFactory(stPacketRepository) {
  /**
   * Store ST Packet
   *
   * @typedef addSTPacket
   * @param {STPacket} packet
   * @return {Promise<CID>}
   */
  async function addSTPacket(packet) {
    return stPacketRepository.store(packet);
  }

  return addSTPacket;
};

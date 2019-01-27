/**
 * @param {STPacketIpfsRepository} stPacketRepository
 * @return {removeSTPacket}
 */
module.exports = function removeSTPacketFactory(stPacketRepository) {
  /**
   * Remove ST Packet
   *
   * @typedef removeSTPacket
   * @param {CID} cid
   * @return {Promise<void>}
   */
  async function removeSTPacket(cid) {
    return stPacketRepository.delete(cid);
  }

  return removeSTPacket;
};

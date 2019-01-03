/**
 * @param {StateTransitionPacketIpfsRepository} stPacketRepository
 * @return {removeSTPacket}
 */
module.exports = function removeSTPacketFactory(stPacketRepository) {
  /**
   * Unpin State Transition packet from IPFS
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

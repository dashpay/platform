const InvalidSTPacketDataError = require('./errors/InvalidSTPacketDataError');

/**
 * @param {STPacketIpfsRepository} stPacketRepository
 * @param {DashPlatformProtocol} dpp
 * @return {addSTPacket}
 */
module.exports = function addSTPacketFactory(stPacketRepository, dpp) {
  /**
   * Store ST Packet
   *
   * @typedef addSTPacket
   * @param {STPacket} stPacket
   * @param {StateTransition|Transaction} stateTransition
   * @return {Promise<CID>}
   */
  async function addSTPacket(stPacket, stateTransition) {
    const result = await dpp.packet.verify(stPacket, stateTransition);

    if (!result.isValid()) {
      throw new InvalidSTPacketDataError(stPacket, stateTransition, result.getErrors());
    }

    return stPacketRepository.store(stPacket);
  }

  return addSTPacket;
};

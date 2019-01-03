const StateTransitionPacket = require('../../storage/stPacket/StateTransitionPacket');

const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {removeSTPacket} removeSTPacket
 * @return {removeSTPacketMethod}
 */
module.exports = function removeSTPacketMethodFactory(removeSTPacket) {
  /**
   * @typedef removeSTPacketMethod
   * @param params
   * @param {string} params.packetHash
   * @return {Promise<void>}
   * @throws {InvalidParamsError}
   */
  async function removeSTPacketMethod(params) {
    if (!Object.prototype.hasOwnProperty.call(params, 'packetHash')) {
      throw new InvalidParamsError('Param "packetHash" is required');
    }

    let cid;
    try {
      cid = StateTransitionPacket.createCIDFromHash(params.packetHash);
    } catch (e) {
      if (e.name === 'InvalidHashError') {
        throw new InvalidParamsError(`Invalid "packetHash" param: ${e.message}`);
      }
      throw e;
    }

    try {
      await removeSTPacket(cid);
    } catch (e) {
      if (e.name === 'PacketNotPinnedError') {
        throw new InvalidParamsError(`Invalid "packetHash" param: ${e.message}`);
      }
      throw e;
    }
  }

  return removeSTPacketMethod;
};

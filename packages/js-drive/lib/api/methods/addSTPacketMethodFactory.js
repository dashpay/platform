const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');

const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {addSTPacket} addSTPacket
 * @param {DashPlatformProtocol} dpp
 *
 * @return {addSTPacketMethod}
 */
module.exports = function addSTPacketMethodFactory(addSTPacket, dpp) {
  /**
   * @typedef addSTPacketMethod
   * @param params
   * @param {string} params.packet
   * @return {Promise<string>}
   * @throws {InvalidParamsError}
   */
  async function addSTPacketMethod(params) {
    if (!Object.prototype.hasOwnProperty.call(params, 'packet')) {
      throw new InvalidParamsError('Param "packet" is required');
    }

    let stPacket;
    try {
      stPacket = await dpp.packet.createFromSerialized(params.packet);
    } catch (e) {
      if (e instanceof InvalidSTPacketError) {
        throw new InvalidParamsError(`Invalid "packet" param: ${e.message}`, e.getErrors());
      }

      throw e;
    }

    const cid = await addSTPacket(stPacket);

    return cid.toBaseEncodedString();
  }

  return addSTPacketMethod;
};

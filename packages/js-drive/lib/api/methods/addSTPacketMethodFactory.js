const StateTransitionPacket = require('../../storage/StateTransitionPacket');
const InvalidParamsError = require('../InvalidParamsError');
const cbor = require('cbor');

/**
 * @param {addSTPacket} addSTPacket
 * @return {addSTPacketMethod}
 */
module.exports = function addSTPacketMethodFactory(addSTPacket) {
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

    let packet;
    try {
      const unserializedPacket = cbor.decodeFirstSync(params.packet);

      packet = new StateTransitionPacket(unserializedPacket);
    } catch (e) {
      throw new InvalidParamsError(`Invalid "packet" param: ${e.message}`);
    }

    const cid = await addSTPacket(packet);

    return cid.toBaseEncodedString();
  }

  return addSTPacketMethod;
};

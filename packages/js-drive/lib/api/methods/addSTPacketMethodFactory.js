const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');

const StateTransition = require('../../blockchain/StateTransition');

const InvalidParamsError = require('../InvalidParamsError');
const InvalidSTPacketDataError = require('../../storage/stPacket/errors/InvalidSTPacketDataError');

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
   * @param {string} params.stPacket
   * @param {string} params.stateTransition
   */
  async function addSTPacketMethod(params) {
    if (!Object.prototype.hasOwnProperty.call(params, 'stPacket')) {
      throw new InvalidParamsError('Param "stPacket" is required');
    }

    if (!Object.prototype.hasOwnProperty.call(params, 'stateTransition')) {
      throw new InvalidParamsError('Param "stateTransition" is required');
    }

    let stPacket;
    try {
      stPacket = await dpp.packet.createFromSerialized(params.stPacket);
    } catch (e) {
      if (e instanceof InvalidSTPacketError) {
        throw new InvalidParamsError(`Invalid "stPacket" param: ${e.message}`, e.getErrors());
      }

      throw e;
    }

    let stateTransition;
    try {
      stateTransition = new StateTransition(params.stateTransition);
    } catch (e) {
      throw new InvalidParamsError(`Invalid "stateTransition" param: ${e.message}`);
    }

    try {
      await addSTPacket(stPacket, stateTransition);
    } catch (e) {
      if (e instanceof InvalidSTPacketDataError) {
        throw new InvalidParamsError(`Invalid "stPacket" and "stateTransition" params: ${e.message}`, e.getErrors());
      }

      throw e;
    }
  }

  return addSTPacketMethod;
};

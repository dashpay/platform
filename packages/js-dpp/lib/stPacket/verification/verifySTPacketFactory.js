const ValidationResult = require('../../validation/ValidationResult');

const UnconfirmedUserError = require('../../errors/UnconfirmedUserError');
const UserNotFoundError = require('../../errors/UserNotFoundError');

/**
 * @param {verifyDapContract} verifyDapContract
 * @param {verifyDapObjects} verifyDapObjects
 * @param {AbstractDataProvider} dataProvider
 * @return {verifySTPacket}
 */
function verifySTPacketFactory(verifyDapContract, verifyDapObjects, dataProvider) {
  /**
   * @typedef verifySTPacket
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   * @return {ValidationResult}
   */
  async function verifySTPacket(stPacket, stateTransition) {
    const result = new ValidationResult();

    const userId = stateTransition.extraPayload.regTxId;

    const registrationTransaction = await dataProvider.fetchTransaction(userId);

    if (!registrationTransaction) {
      result.addError(
        new UserNotFoundError(userId),
      );
    } else if (registrationTransaction.confirmations < 6) {
      result.addError(
        new UnconfirmedUserError(registrationTransaction),
      );
    }

    if (stPacket.getDapContract()) {
      result.merge(
        await verifyDapContract(stPacket),
      );
    }

    if (stPacket.getDapObjects().length) {
      result.merge(
        await verifyDapObjects(stPacket, userId),
      );
    }

    return result;
  }

  return verifySTPacket;
}

module.exports = verifySTPacketFactory;

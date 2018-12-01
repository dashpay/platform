const VerificationResult = require('./VerificationResult');

const EmptySTPacketError = require('../../consensusErrors/EmptySTPacketError');
const ConsensusError = require('../../consensusErrors/ConsensusError');

/**
 * @param {verifyDapContract} verifyDapContract
 * @param {verifyDapObjects} verifyDapObjects
 * @return {verifySTPacket}
 */
function verifySTPacketFactory(verifyDapContract, verifyDapObjects) {
  /**
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   * @param {AbstractDataProvider} dataProvider
   * @return {Promise<VerificationResult>}
   */
  async function verifySTPacket(stPacket, stateTransition, dataProvider) {
    const result = new VerificationResult();

    const blockChainUserId = stateTransition.extraPayload.regTxId;

    const registrationTransaction = dataProvider.getTransaction(blockChainUserId);
    if (registrationTransaction.confirmations < 6) {
      result.addError(new ConsensusError('Blockchain user has less than 6 confirmation'));

      return result;
    }

    if (stPacket.getDapContract()) {
      return verifyDapContract(stPacket, dataProvider);
    }

    if (stPacket.getDapObjects().length) {
      return verifyDapObjects(stPacket, dataProvider);
    }

    result.addError(new EmptySTPacketError());

    return result;
  }

  return verifySTPacket;
}

module.exports = verifySTPacketFactory;

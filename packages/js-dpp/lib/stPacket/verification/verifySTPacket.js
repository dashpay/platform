const VerificationResult = require('./VerificationResult');

const EmptySTPacketError = require('../../consensusErrors/EmptySTPacketError');

/**
 * @param {verifyDapContract} verifyDapContract
 * @param {verifyDapObjects} verifyDapObjects
 * @return {verifySTPacket}
 */
function verifySTPacketFactory(verifyDapContract, verifyDapObjects) {
  /**
   * @param {STPacket} stPacket
   * @param {AbstractDataProvider} dataProvider
   * @return {Promise<VerificationResult>}
   */
  async function verifySTPacket(stPacket, dataProvider) {
    const result = new VerificationResult();

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

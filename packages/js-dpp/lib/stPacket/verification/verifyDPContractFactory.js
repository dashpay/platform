const ValidationResult = require('../../validation/ValidationResult');

const DPContractAlreadyPresentError = require('../../errors/DPContractAlreadyPresentError');

/**
 *
 * @param {DataProvider} dataProvider
 * @return {verifyDPContract}
 */
function verifyDPContractFactory(dataProvider) {
  /**
   * @typedef verifyDPContract
   * @param {STPacket} stPacket
   * @return {ValidationResult}
   */
  async function verifyDPContract(stPacket) {
    const result = new ValidationResult();

    const dpContract = await dataProvider.fetchDPContract(stPacket.getDPContractId());

    if (dpContract) {
      result.addError(
        new DPContractAlreadyPresentError(stPacket.getDPContract()),
      );
    }

    return result;
  }

  return verifyDPContract;
}

module.exports = verifyDPContractFactory;

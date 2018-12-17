const ValidationResult = require('../../validation/ValidationResult');

const DapContractAlreadyPresentError = require('../../errors/DapContractAlreadyPresentError');

/**
 *
 * @param {AbstractDataProvider} dataProvider
 * @return {verifyDapContract}
 */
function verifyDapContractFactory(dataProvider) {
  /**
   * @typedef verifyDapContract
   * @param {STPacket} stPacket
   * @return {ValidationResult}
   */
  async function verifyDapContract(stPacket) {
    const result = new ValidationResult();

    const dapContract = await dataProvider.fetchDapContract(stPacket.getDapContractId());

    if (dapContract) {
      result.addError(
        new DapContractAlreadyPresentError(stPacket.getDapContract()),
      );
    }

    return result;
  }

  return verifyDapContract;
}

module.exports = verifyDapContractFactory;

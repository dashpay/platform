const ValidationResult = require('../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../errors/DataContractAlreadyPresentError');

/**
 * @typedef verifyContract
 * @param {STPacket} stPacket
 * @param {DataContract} contract
 * @return {ValidationResult}
 */
async function verifyContract(stPacket, contract) {
  const result = new ValidationResult();

  if (contract) {
    result.addError(
      new DataContractAlreadyPresentError(stPacket.getContract()),
    );
  }

  return result;
}

module.exports = verifyContract;

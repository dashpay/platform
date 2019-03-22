const ValidationResult = require('../../validation/ValidationResult');

const ContractAlreadyPresentError = require('../../errors/ContractAlreadyPresentError');

/**
 * @typedef verifyContract
 * @param {STPacket} stPacket
 * @param {Contract} contract
 * @return {ValidationResult}
 */
async function verifyContract(stPacket, contract) {
  const result = new ValidationResult();

  if (contract) {
    result.addError(
      new ContractAlreadyPresentError(stPacket.getContract()),
    );
  }

  return result;
}

module.exports = verifyContract;

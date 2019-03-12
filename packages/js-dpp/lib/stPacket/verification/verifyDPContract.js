const ValidationResult = require('../../validation/ValidationResult');

const DPContractAlreadyPresentError = require('../../errors/DPContractAlreadyPresentError');

/**
 * @typedef verifyDPContract
 * @param {STPacket} stPacket
 * @param {DPContract} dpContract
 * @return {ValidationResult}
 */
async function verifyDPContract(stPacket, dpContract) {
  const result = new ValidationResult();

  if (dpContract) {
    result.addError(
      new DPContractAlreadyPresentError(stPacket.getDPContract()),
    );
  }

  return result;
}

module.exports = verifyDPContract;

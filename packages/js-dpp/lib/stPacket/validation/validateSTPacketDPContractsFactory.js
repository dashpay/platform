const InvalidSTPacketContractIdError = require('../../errors/InvalidSTPacketContractIdError');

/**
 * @param {validateDPContract} validateDPContract
 * @param {createDPContract} createDPContract
 * @return {validateSTPacketDPContracts}
 */
function validateSTPacketDPContractsFactory(validateDPContract, createDPContract) {
  /**
   * @typedef validateSTPacketDPContracts
   * @param {Object} rawSTPacket
   * @return {ValidationResult}
   */
  function validateSTPacketDPContracts(rawSTPacket) {
    const { contracts: [rawDPContract] } = rawSTPacket;

    const result = validateDPContract(rawDPContract);

    if (!result.isValid()) {
      return result;
    }

    const dpContract = createDPContract(rawDPContract);

    if (rawSTPacket.contractId !== dpContract.getId()) {
      result.addError(
        new InvalidSTPacketContractIdError(rawSTPacket.contractId, dpContract),
      );
    }

    return result;
  }

  return validateSTPacketDPContracts;
}

module.exports = validateSTPacketDPContractsFactory;

const InvalidSTPacketContractIdError = require('../../errors/InvalidSTPacketContractIdError');

/**
 * @param {validateDapContract} validateDapContract
 * @param {createDapContract} createDapContract
 * @return {validateSTPacketDapContracts}
 */
function validateSTPacketDapContractsFactory(validateDapContract, createDapContract) {
  /**
   * @typedef validateSTPacketDapContracts
   * @param {Object} rawSTPacket
   * @return {ValidationResult}
   */
  function validateSTPacketDapContracts(rawSTPacket) {
    const { contracts: [rawDapContract] } = rawSTPacket;

    const result = validateDapContract(rawDapContract);

    if (!result.isValid()) {
      return result;
    }

    const dapContract = createDapContract(rawDapContract);

    if (rawSTPacket.contractId !== dapContract.getId()) {
      result.addError(
        new InvalidSTPacketContractIdError(rawSTPacket.contractId, dapContract),
      );
    }

    return result;
  }

  return validateSTPacketDapContracts;
}

module.exports = validateSTPacketDapContractsFactory;

const InvalidSTPacketContractIdError = require('../../errors/InvalidSTPacketContractIdError');

/**
 * @param {validateDapContract} validateDapContract
 * @param {createDapContract} createDapContract
 * @return {validateSTPacketDapContracts}
 */
function validateSTPacketDapContractsFactory(validateDapContract, createDapContract) {
  /**
   * @typedef validateSTPacketDapContracts
   * @param {Object[]} rawDapContracts
   * @param {Object} rawStPacket
   * @return {ValidationResult}
   */
  function validateSTPacketDapContracts(rawDapContracts, rawStPacket) {
    const [rawDapContract] = rawDapContracts;

    const result = validateDapContract(rawDapContract);

    if (!result.isValid()) {
      return result;
    }

    const dapContract = createDapContract(rawDapContract);

    if (rawStPacket.contractId !== dapContract.getId()) {
      result.addError(
        new InvalidSTPacketContractIdError(rawStPacket.contractId, dapContract),
      );
    }

    return result;
  }

  return validateSTPacketDapContracts;
}

module.exports = validateSTPacketDapContractsFactory;

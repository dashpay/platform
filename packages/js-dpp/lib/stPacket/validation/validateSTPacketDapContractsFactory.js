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
   * @param {ValidationResult} result
   */
  function validateSTPacketDapContracts(rawDapContracts, rawStPacket, result) {
    const [rawDapContract] = rawDapContracts;

    const dapContractResult = validateDapContract(rawDapContract);

    if (!dapContractResult.isValid()) {
      result.merge(dapContractResult);

      return;
    }

    const dapContract = createDapContract(rawDapContract);

    if (rawStPacket.contractId !== dapContract.getId()) {
      result.addError(
        new InvalidSTPacketContractIdError(rawStPacket.contractId, dapContract),
      );
    }
  }

  return validateSTPacketDapContracts;
}

module.exports = validateSTPacketDapContractsFactory;

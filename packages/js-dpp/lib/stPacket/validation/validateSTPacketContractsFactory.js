/**
 * @param {validateContract} validateContract
 * @return {validateSTPacketContracts}
 */
function validateSTPacketContractsFactory(validateContract) {
  /**
   * @typedef validateSTPacketContracts
   * @param {RawSTPacket} rawSTPacket
   * @return {ValidationResult}
   */
  function validateSTPacketContracts(rawSTPacket) {
    const { contracts: [rawContract] } = rawSTPacket;

    return validateContract(rawContract);
  }

  return validateSTPacketContracts;
}

module.exports = validateSTPacketContractsFactory;

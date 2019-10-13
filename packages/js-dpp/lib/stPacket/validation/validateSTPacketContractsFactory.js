/**
 * @param {validateDataContract} validateDataContract
 * @return {validateSTPacketContracts}
 */
function validateSTPacketContractsFactory(validateDataContract) {
  /**
   * @typedef validateSTPacketContracts
   * @param {RawSTPacket} rawSTPacket
   * @return {ValidationResult}
   */
  function validateSTPacketContracts(rawSTPacket) {
    const { contracts: [rawDataContract] } = rawSTPacket;

    return validateDataContract(rawDataContract);
  }

  return validateSTPacketContracts;
}

module.exports = validateSTPacketContractsFactory;

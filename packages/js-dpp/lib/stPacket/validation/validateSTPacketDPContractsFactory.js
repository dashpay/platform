/**
 * @param {validateDPContract} validateDPContract
 * @return {validateSTPacketDPContracts}
 */
function validateSTPacketDPContractsFactory(validateDPContract) {
  /**
   * @typedef validateSTPacketDPContracts
   * @param {Object} rawSTPacket
   * @return {ValidationResult}
   */
  function validateSTPacketDPContracts(rawSTPacket) {
    const { contracts: [rawDPContract] } = rawSTPacket;

    return validateDPContract(rawDPContract);
  }

  return validateSTPacketDPContracts;
}

module.exports = validateSTPacketDPContractsFactory;

const VerificationResult = require('./VerificationResult');

const ConsensusError = require('../../consensusErrors/ConsensusError');

/**
 * @typedef verifyDapContract
 * @param {STPacket} stPacket
 * @param {AbstractDataProvider} dataProvider
 * @return {VerificationResult}
 */
async function verifyDapContract(stPacket, dataProvider) {
  const result = new VerificationResult();

  if (stPacket.getDapContractId() !== stPacket.getDapContract().getId()) {
    const error = new ConsensusError('Dap Contract ID should be equal to contractId');

    result.addError(error);

    return result;
  }

  const dapContract = await dataProvider.fetchDapContract(stPacket.getDapContractId());

  if (dapContract) {
    const error = new ConsensusError('This Dap Contract is already present');

    result.addError(error);
  }

  return result;
}

module.exports = verifyDapContract;

const DapContract = require('./DapContract');

/**
 * @param {DapContractMongoDbRepository} dapContractRepository
 * @returns {updateDapContract}
 */
function updateDapContractFactory(dapContractRepository) {
  /**
   * Generate DAP contract State View
   *
   * @typedef {Promise} updateDapContract
   * @param {string} dapId
   * @param {Reference} reference
   * @param {object} dapContractData
   * @returns {Promise<void>}
   */
  async function updateDapContract(dapId, reference, dapContractData) {
    const { upgradedapid } = dapContractData;

    const currentDapContract = new DapContract(
      dapId,
      dapContractData,
      reference,
      false,
    );

    if (!upgradedapid) {
      await dapContractRepository.store(currentDapContract);
      return;
    }

    const previousDapContract = await dapContractRepository.find(dapId);
    if (!previousDapContract) {
      return;
    }

    currentDapContract.addRevision(previousDapContract);
    await dapContractRepository.store(currentDapContract);
  }

  return updateDapContract;
}

module.exports = updateDapContractFactory;

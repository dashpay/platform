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
   * @param {boolean} reverting
   * @returns {Promise<void>}
   */
  async function updateDapContract(dapId, reference, dapContractData, reverting) {
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

    // NOTE: Since reverting is more complicated
    // `previousDapContract` is actually next one here
    // so we have to remove it's version and the version before that
    // to have a proper set of `previousVersions`
    if (reverting) {
      currentDapContract.removeAheadRevisions();
    }

    await dapContractRepository.store(currentDapContract);
  }

  return updateDapContract;
}

module.exports = updateDapContractFactory;

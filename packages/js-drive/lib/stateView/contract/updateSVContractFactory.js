const SVContract = require('./SVContract');

/**
 * @param {SVContractMongoDbRepository} svContractRepository
 * @returns {updateSVContract}
 */
function updateSVContractFactory(svContractRepository) {
  /**
   * Generate Contract State View
   *
   * @typedef {Promise} updateSVContract
   * @param {string} contractId
   * @param {string} userId
   * @param {Reference} reference
   * @param {Contract} contract
   * @param {boolean} [reverting]
   *
   * @returns {Promise<void>}
   */
  async function updateSVContract(contractId, userId, reference, contract, reverting = false) {
    const currentSVContract = new SVContract(
      contractId,
      userId,
      contract,
      reference,
    );

    const previousSVContract = await svContractRepository.find(contractId);
    if (previousSVContract) {
      currentSVContract.addRevision(previousSVContract);

      // NOTE: Since reverting is more complicated
      // `previousSVContract` is actually next one here
      // so we have to remove it's version and the version before that
      // to have a proper set of `previousRevisions`
      if (reverting) {
        currentSVContract.removeAheadRevisions();
      }
    }

    await svContractRepository.store(currentSVContract);
  }

  return updateSVContract;
}

module.exports = updateSVContractFactory;

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
   * @param {DataContract} contract
   * @param {Reference} reference
   * @param {MongoDBTransaction} [stateViewTransaction]
   *
   * @returns {Promise<SVContract>}
   */
  async function updateSVContract(
    contract,
    reference,
    stateViewTransaction = undefined,
  ) {
    const currentSVContract = new SVContract(
      contract,
      reference,
    );

    const previousSVContract = await svContractRepository.find(
      currentSVContract.getId(),
      stateViewTransaction,
    );

    if (previousSVContract) {
      currentSVContract.addRevision(previousSVContract);
    }

    await svContractRepository.store(currentSVContract, stateViewTransaction);

    return currentSVContract;
  }

  return updateSVContract;
}

module.exports = updateSVContractFactory;

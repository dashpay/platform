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
   * @param {MongoDBTransaction} [transaction]
   *
   * @returns {Promise<void>}
   */
  async function updateSVContract(
    contractId,
    userId,
    reference,
    contract,
    transaction = undefined,
  ) {
    const currentSVContract = new SVContract(
      contractId,
      userId,
      contract,
      reference,
    );

    const previousSVContract = await svContractRepository.find(contractId, transaction);
    if (previousSVContract) {
      currentSVContract.addRevision(previousSVContract);
    }

    await svContractRepository.store(currentSVContract, transaction);
  }

  return updateSVContract;
}

module.exports = updateSVContractFactory;

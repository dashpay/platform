/**
 * @param {SVContractMongoDbRepository} svContractRepository
 * @returns {fetchContract}
 */
function fetchContractFactory(svContractRepository) {
  /**
   * Fetch Contract by id
   *
   * @typedef fetchContract
   * @param {string} contractId
   * @returns {Promise<Contract|null>}
   */
  async function fetchContract(contractId) {
    const svContract = await svContractRepository.find(contractId);

    if (!svContract) {
      return null;
    }

    return svContract.getContract();
  }

  return fetchContract;
}

module.exports = fetchContractFactory;

/**
 * @param {SVContractMongoDbRepository} svContractRepository
 * @returns {fetchDPContract}
 */
function fetchDPContractFactory(svContractRepository) {
  /**
   * Fetch DP Contract by id
   *
   * @typedef fetchDPContract
   * @param {string} contractId
   * @returns {Promise<DPContract|null>}
   */
  async function fetchDPContract(contractId) {
    const svContract = await svContractRepository.find(contractId);

    if (!svContract) {
      return null;
    }

    return svContract.getDPContract();
  }

  return fetchDPContract;
}

module.exports = fetchDPContractFactory;

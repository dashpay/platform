/**
 * @param {DapContractMongoDbRepository} dapContractMongoDbRepository
 * @returns {fetchDapContract}
 */
function fetchDapContractFactory(dapContractMongoDbRepository) {
  /**
   * Fetch Dap Contract by DAP id
   *
   * @typedef fetchDapContract
   * @param {string} dapId
   * @returns {Promise<Object|null>}
   */
  async function fetchDapContract(dapId) {
    const dapContract = await dapContractMongoDbRepository.find(dapId);
    if (!dapContract) {
      return null;
    }
    return dapContract.getOriginalData();
  }

  return fetchDapContract;
}

module.exports = fetchDapContractFactory;

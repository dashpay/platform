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
    const { dapname, schema } = dapContractData;
    const dapContract = new DapContract(dapId, dapname, reference, schema);
    await dapContractRepository.store(dapContract);
  }

  return updateDapContract;
}

module.exports = updateDapContractFactory;

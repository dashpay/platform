const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {DapContractMongoDbRepository} dapContractRepository
 * @returns {fetchDapContractMethod}
 */
module.exports = function fetchDapContractMethodFactory(dapContractRepository) {
  /**
   * @typedef fetchDapContractMethod
   * @param {string} dapId
   * @returns {Promise<object>}
   */
  async function fetchDapContractMethod(dapId) {
    if (!dapId) {
      throw new InvalidParamsError();
    }

    const dapContract = dapContractRepository.find(dapId);
    return dapContract.toJSON();
  }

  return fetchDapContractMethod;
};

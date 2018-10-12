const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {DapContractMongoDbRepository} dapContractRepository
 * @returns {fetchDapContractMethod}
 */
module.exports = function fetchDapContractMethodFactory(dapContractRepository) {
  /**
   * @typedef fetchDapContractMethod
   * @param {{ dapId: string }} params
   * @returns {Promise<object>}
   */
  async function fetchDapContractMethod(params) {
    if (!params.dapId) {
      throw new InvalidParamsError();
    }

    const dapContract = await dapContractRepository.find(params.dapId);
    return dapContract.toJSON();
  }

  return fetchDapContractMethod;
};

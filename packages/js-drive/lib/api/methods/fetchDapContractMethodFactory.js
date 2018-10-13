const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {DapContractMongoDbRepository} dapContractRepository
 * @returns {fetchDapContractMethod}
 */
module.exports = function fetchDapContractMethodFactory(dapContractRepository) {
  /**
   * @typedef fetchDapContractMethod
   * @param {{ dapId: string }} params
   * @throws InvalidParamsError
   * @returns {Promise<object>}
   */
  async function fetchDapContractMethod(params) {
    if (!params.dapId) {
      throw new InvalidParamsError("'dapId' param is not present");
    }

    const dapContract = await dapContractRepository.find(params.dapId);

    if (!dapContract) {
      throw new InvalidParamsError('Dap Contract not found');
    }

    return dapContract.toJSON();
  }

  return fetchDapContractMethod;
};

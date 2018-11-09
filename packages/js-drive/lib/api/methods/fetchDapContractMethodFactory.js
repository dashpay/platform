const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {fetchDapContract} fetchDapContract
 * @returns {fetchDapContractMethod}
 */
module.exports = function fetchDapContractMethodFactory(fetchDapContract) {
  /**
   * @typedef fetchDapContractMethod
   * @param {{ dapId: string }} params
   * @throws InvalidParamsError
   * @returns {Promise<Object>}
   */
  async function fetchDapContractMethod(params) {
    if (!params.dapId) {
      throw new InvalidParamsError("'dapId' param is not present");
    }

    const dapContract = await fetchDapContract(params.dapId);

    if (!dapContract) {
      throw new InvalidParamsError('Dap Contract not found');
    }

    return dapContract;
  }

  return fetchDapContractMethod;
};

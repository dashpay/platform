const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {fetchDPContract} fetchDPContract
 * @returns {fetchDPContractMethod}
 */
module.exports = function fetchDPContractMethodFactory(fetchDPContract) {
  /**
   * @typedef fetchDPContractMethod
   * @param {{ contractId: string }} params
   * @throws InvalidParamsError
   * @returns {Object}
   */
  async function fetchDPContractMethod(params) {
    if (!params.contractId) {
      throw new InvalidParamsError("'contractId' param is not present");
    }

    const dpContract = await fetchDPContract(params.contractId);

    if (!dpContract) {
      throw new InvalidParamsError('DP Contract not found');
    }

    return dpContract.toJSON();
  }

  return fetchDPContractMethod;
};

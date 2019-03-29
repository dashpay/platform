const InvalidParamsError = require('../InvalidParamsError');

/**
 * @param {fetchContract} fetchContract
 * @returns {fetchContractMethod}
 */
module.exports = function fetchContractMethodFactory(fetchContract) {
  /**
   * @typedef fetchContractMethod
   * @param {{ contractId: string }} params
   * @throws InvalidParamsError
   * @returns {Object}
   */
  async function fetchContractMethod(params) {
    if (!params.contractId) {
      throw new InvalidParamsError("'contractId' param is not present");
    }

    const contract = await fetchContract(params.contractId);

    if (!contract) {
      throw new InvalidParamsError('Contract not found');
    }

    return contract.toJSON();
  }

  return fetchContractMethod;
};

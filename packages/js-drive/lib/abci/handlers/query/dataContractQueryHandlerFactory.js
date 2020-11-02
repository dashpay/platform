const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {DataContractStoreRepository} dataContractRepository
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(dataContractRepository) {
  /**
   * @typedef dataContractQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.id
   * @return {Promise<ResponseQuery>}
   */
  async function dataContractQueryHandler(params, { id }) {
    const dataContract = await dataContractRepository.fetch(id);

    if (!dataContract) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    return new ResponseQuery({
      value: dataContract.toBuffer(),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

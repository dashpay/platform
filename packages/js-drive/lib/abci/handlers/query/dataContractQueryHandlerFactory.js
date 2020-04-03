const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {DataContractLevelDBRepository} dataContractRepository
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(dataContractRepository) {
  /**
   * @typedef dataContractQueryHandler
   * @param {Object} params
   * @param {string} params.id
   * @return {Promise<ResponseQuery>}
   */
  async function dataContractQueryHandler({ id }) {
    const dataContract = await dataContractRepository.fetch(id);

    if (!dataContract) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    return new ResponseQuery({
      value: dataContract.serialize(),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

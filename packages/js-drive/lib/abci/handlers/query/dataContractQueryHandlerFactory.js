const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const cbor = require('cbor');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {RootTree} rootTree
 * @param {DataContractsStoreRootTreeLeaf} dataContractsStoreRootTreeLeaf
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(
  dataContractRepository,
  rootTree,
  dataContractsStoreRootTreeLeaf,
) {
  /**
   * @typedef dataContractQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.id
   * @param {Object} request
   * @param {boolean} [request.prove]
   * @return {Promise<ResponseQuery>}
   */
  async function dataContractQueryHandler(params, { id }, request) {
    const dataContract = await dataContractRepository.fetch(id);

    if (!dataContract) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    const includeProof = request.prove === 'true';

    const value = {
      data: dataContract.toBuffer(),
    };

    if (includeProof) {
      value.proof = rootTree.getFullProof(dataContractsStoreRootTreeLeaf, [id]);
    }

    return new ResponseQuery({
      value: cbor.encode(value),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

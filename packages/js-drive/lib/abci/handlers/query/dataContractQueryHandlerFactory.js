const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {DataContractStoreRepository} previousDataContractRepository
 * @param {RootTree} previousRootTree
 * @param {DataContractsStoreRootTreeLeaf} previousDataContractsStoreRootTreeLeaf
 * @param {createQueryResponse} createQueryResponse
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(
  previousDataContractRepository,
  previousRootTree,
  previousDataContractsStoreRootTreeLeaf,
  createQueryResponse,
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
    const isProofRequested = request.prove === 'true';

    const response = createQueryResponse(GetDataContractResponse, isProofRequested);

    const dataContract = await previousDataContractRepository.fetch(id);

    if (!dataContract) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    if (isProofRequested) {
      const proof = response.getProof();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProof(previousDataContractsStoreRootTreeLeaf, [id]);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProof(storeTreeProof);
    } else {
      response.setDataContract(dataContract.toBuffer());
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

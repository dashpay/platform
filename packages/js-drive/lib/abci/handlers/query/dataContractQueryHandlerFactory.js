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
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');

/**
 *
 * @param {DataContractStoreRepository} previousDataContractRepository
 * @param {RootTree} previousRootTree
 * @param {DataContractsStoreRootTreeLeaf} previousDataContractsStoreRootTreeLeaf
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(
  previousDataContractRepository,
  previousRootTree,
  previousDataContractsStoreRootTreeLeaf,
  createQueryResponse,
  blockExecutionContext,
  previousBlockExecutionContext,
) {
  /**
   * @typedef dataContractQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.id
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function dataContractQueryHandler(params, { id }, request) {
    // There is no signed state (current committed block height less then 2)
    if (blockExecutionContext.isEmpty() || previousBlockExecutionContext.isEmpty()) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    const response = createQueryResponse(GetDataContractResponse, request.prove);

    const dataContract = await previousDataContractRepository.fetch(id);

    let dataContractBuffer;
    if (!dataContract && !request.prove) {
      throw new NotFoundAbciError('Data Contract not found');
    } else if (dataContract) {
      dataContractBuffer = dataContract.toBuffer();
    }

    if (request.prove) {
      const proof = response.getProof();
      const storeTreeProofs = new StoreTreeProofs();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProofForOneLeaf(previousDataContractsStoreRootTreeLeaf, [id]);

      storeTreeProofs.setDataContractsProof(storeTreeProof);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
    } else {
      response.setDataContract(dataContractBuffer);
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

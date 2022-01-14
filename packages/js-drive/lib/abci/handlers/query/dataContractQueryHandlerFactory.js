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
const UnimplementedAbciError = require('../../errors/UnimplementedAbciError');

/**
 *
 * @param {DataContractStoreRepository} signedDataContractRepository
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {dataContractQueryHandler}
 */
function dataContractQueryHandlerFactory(
  signedDataContractRepository,
  createQueryResponse,
  blockExecutionContextStack,
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
    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    const response = createQueryResponse(GetDataContractResponse, request.prove);

    if (request.prove) {
      throw new UnimplementedAbciError('Proofs are not implemented yet');
    }

    const dataContract = await signedDataContractRepository.fetch(id);

    if (dataContract) {
      throw new NotFoundAbciError('Data Contract not found');
    }

    response.setDataContract(dataContract.toBuffer());

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return dataContractQueryHandler;
}

module.exports = dataContractQueryHandlerFactory;

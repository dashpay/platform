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

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');

const NotFoundAbciError = require('../../errors/NotFoundAbciError');
const UnimplementedAbciError = require('../../errors/UnimplementedAbciError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

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

    let contractIdIdentifier;
    try {
      contractIdIdentifier = new Identifier(id);
    } catch (e) {
      if (e instanceof IdentifierError) {
        throw new InvalidArgumentAbciError('id must be a valid identifier (32 bytes long)');
      }

      throw e;
    }

    const response = createQueryResponse(GetDataContractResponse, request.prove);

    if (request.prove) {
      throw new UnimplementedAbciError('Proofs are not implemented yet');
    }

    const dataContract = await signedDataContractRepository.fetch(contractIdIdentifier);

    if (!dataContract) {
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

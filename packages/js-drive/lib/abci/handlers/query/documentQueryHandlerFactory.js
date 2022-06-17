const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetDocumentsResponse,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const InvalidQueryError = require('../../../document/errors/InvalidQueryError');

/**
 *
 * @param {fetchDocuments} fetchSignedDocuments
 * @param {fetchDataContract} fetchSignedDataContract
 * @param {proveDocuments} proveSignedDocuments
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchSignedDocuments,
  fetchSignedDataContract,
  proveSignedDocuments,
  createQueryResponse,
  blockExecutionContextStack,
) {
  /**
   * @typedef {documentQueryHandler}
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.contractId
   * @param {string} data.type
   * @param {string} [data.where]
   * @param {string} [data.orderBy]
   * @param {string} [data.limit]
   * @param {Buffer} [data.startAfter]
   * @param {Buffer} [data.startAt]
   * @param {RequestQuery} request
   * @return {Promise<ResponseQuery>}
   */
  async function documentQueryHandler(
    params,
    {
      contractId,
      type,
      where,
      orderBy,
      limit,
      startAfter,
      startAt,
    },
    request,
  ) {
    // There is no signed state (current committed block height less than 3)
    if (!blockExecutionContextStack.getLast()) {
      const response = new GetDocumentsResponse();

      response.setMetadata(new ResponseMetadata());

      return new ResponseQuery({
        value: response.serializeBinary(),
      });
    }

    const response = createQueryResponse(GetDocumentsResponse, request.prove);

    let documentsResult;
    let dataContractResult;
    let proof;
    const options = {
      where,
      orderBy,
      limit,
      startAfter: startAfter ? Buffer.from(startAfter) : startAfter,
      startAt: startAt ? Buffer.from(startAt) : startAt,
    };

    try {
      dataContractResult = await fetchSignedDataContract(contractId, type);

      documentsResult = await fetchSignedDocuments(dataContractResult, type, options);

      if (request.prove) {
        proof = await proveSignedDocuments(dataContractResult, type, options);
      }
    } catch (e) {
      if (e instanceof InvalidQueryError) {
        throw new InvalidArgumentAbciError(`Invalid query: ${e.message}`);
      }

      throw e;
    }

    const documents = documentsResult.getValue();

    response.setDocumentsList(
      documents.map((document) => document.toBuffer()),
    );

    if (request.prove) {
      response.getProof().setMerkleProof(proof.getValue());
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

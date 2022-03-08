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
const UnimplementedAbciError = require('../../errors/UnimplementedAbciError');

/**
 *
 * @param {fetchDocuments} fetchSignedDocuments
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchSignedDocuments,
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

    if (request.prove) {
      throw new UnimplementedAbciError('Proofs are not implemented yet');
    }

    let documents;

    try {
      documents = await fetchSignedDocuments(contractId, type, {
        where,
        orderBy,
        limit,
        startAfter: startAfter ? Buffer.from(startAfter) : startAfter,
        startAt: startAt ? Buffer.from(startAt) : startAt,
      });
    } catch (e) {
      if (e instanceof InvalidQueryError) {
        throw new InvalidArgumentAbciError(
          `Invalid query: ${e.getErrors()[0].message}`,
          { errors: e.getErrors() },
        );
      }

      throw e;
    }

    response.setDocumentsList(
      documents.map((document) => document.toBuffer()),
    );

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

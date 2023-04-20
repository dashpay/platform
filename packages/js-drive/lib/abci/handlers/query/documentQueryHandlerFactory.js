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
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const InvalidQueryError = require('../../../document/errors/InvalidQueryError');

/**
 *
 * @param {fetchDocuments} fetchDocuments
 * @param {proveDocuments} proveDocuments
 * @param {createQueryResponse} createQueryResponse
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchDocuments,
  proveDocuments,
  createQueryResponse,
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
    const response = createQueryResponse(GetDocumentsResponse, request.prove);

    const options = {
      where,
      orderBy,
      limit,
      startAfter: startAfter ? Buffer.from(startAfter) : startAfter,
      startAt: startAt ? Buffer.from(startAt) : startAt,
    };

    try {
      if (request.prove) {
        const proof = await proveDocuments(contractId, type, options);

        response.getProof().setMerkleProof(proof.getValue());
      } else {
        const documentsResult = await fetchDocuments(contractId, type, options);

        const documents = documentsResult.getValue();

        response.setDocumentsList(
          documents.map((document) => document.toBuffer()),
        );
      }
    } catch (e) {
      if (e instanceof InvalidQueryError) {
        throw new InvalidArgumentAbciError(`Invalid query: ${e.message}`);
      }

      throw e;
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

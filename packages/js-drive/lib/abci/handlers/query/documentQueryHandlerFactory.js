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
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const UnavailableAbciError = require('../../errors/UnavailableAbciError');
const InvalidQueryError = require('../../../document/errors/InvalidQueryError');

/**
 *
 * @param {fetchDocuments} fetchSignedDocuments
 * @param {AwilixContainer} container
 * @param {createQueryResponse} createQueryResponse
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchSignedDocuments,
  container,
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
   * @param {string} [data.startAfter]
   * @param {string} [data.startAt]
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

    let documents;

    try {
      documents = await fetchSignedDocuments(contractId, type, {
        where,
        orderBy,
        limit,
        startAfter,
        startAt,
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

    if (request.prove) {
      const documentIds = documents.map((document) => document.getId());

      const proof = response.getProof();
      const storeTreeProofs = new StoreTreeProofs();

      const {
        rootTreeProof,
        storeTreeProof,
      } = previousRootTree.getFullProofForOneLeaf(previousDocumentsStoreRootTreeLeaf, documentIds);

      storeTreeProofs.setDocumentsProof(storeTreeProof);

      proof.setRootTreeProof(rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
    } else {
      response.setDocumentsList(documents.map((document) => document.toBuffer()));
    }

    return new ResponseQuery({
      value: response.serializeBinary(),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

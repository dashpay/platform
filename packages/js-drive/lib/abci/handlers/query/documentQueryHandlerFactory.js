const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');

const InvalidQueryError = require('../../../document/errors/InvalidQueryError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const UnavailableAbciError = require('../../errors/UnavailableAbciError');

/**
 *
 * @param {fetchDocuments} fetchPreviousDocuments
 * @param {RootTree} previousRootTree
 * @param {DocumentsStoreRootTreeLeaf} previousDocumentsStoreRootTreeLeaf
 * @param {AwilixContainer} container
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchPreviousDocuments,
  previousRootTree,
  previousDocumentsStoreRootTreeLeaf,
  container,
) {
  /**
   * @typedef documentQueryHandler
   * @param {Object} params
   * @param {Object} data
   * @param {Buffer} data.contractId
   * @param {string} data.type
   * @param {string} [data.where]
   * @param {string} [data.orderBy]
   * @param {string} [data.limit]
   * @param {string} [data.startAfter]
   * @param {string} [data.startAt]
   * @param {Object} request
   * @param {boolean} [request.prove]
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
    if (!container.has('previousBlockExecutionStoreTransactions')) {
      throw new UnavailableAbciError();
    }

    const previousBlockExecutionTransactions = container.resolve('previousBlockExecutionStoreTransactions');
    const dataContractTransaction = previousBlockExecutionTransactions.getTransaction('dataContracts');
    if (!dataContractTransaction.isStarted()) {
      throw new UnavailableAbciError();
    }

    let documents;

    try {
      documents = await fetchPreviousDocuments(contractId, type, {
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

    const includeProof = request.prove === 'true';

    const value = {
      data: documents.map((document) => document.toBuffer()),
    };

    if (includeProof) {
      const documentIds = documents.map((document) => document.getId());

      value.proof = previousRootTree.getFullProof(previousDocumentsStoreRootTreeLeaf, documentIds);
    }

    return new ResponseQuery({
      value: await cbor.encodeAsync(
        value,
      ),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

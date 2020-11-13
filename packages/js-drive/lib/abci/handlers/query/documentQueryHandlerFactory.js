const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const cbor = require('cbor');

const InvalidQueryError = require('../../../document/errors/InvalidQueryError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {fetchDocuments} fetchDocuments
 * @param {RootTree} rootTree
 * @param {DocumentsStoreRootTreeLeaf} documentsStoreRootTreeLeaf
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(
  fetchDocuments,
  rootTree,
  documentsStoreRootTreeLeaf,
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
    let documents;

    try {
      documents = await fetchDocuments(contractId, type, {
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

      value.proof = rootTree.getFullProof(documentsStoreRootTreeLeaf, documentIds);
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

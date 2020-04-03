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
 * @return {documentQueryHandler}
 */
function documentQueryHandlerFactory(fetchDocuments) {
  /**
   * @typedef documentQueryHandler
   * @param {Object} params
   * @param {string} params.contractId
   * @param {string} params.type
   * @param {Object} options
   * @return {Promise<ResponseQuery>}
   */
  async function documentQueryHandler({ contractId, type }, options) {
    let documents;

    try {
      documents = await fetchDocuments(contractId, type, options);
    } catch (e) {
      if (e instanceof InvalidQueryError) {
        throw new InvalidArgumentAbciError(
          `Invalid query: ${e.getErrors()[0].message}`,
          { errors: e.getErrors() },
        );
      }

      throw e;
    }

    return new ResponseQuery({
      value: await cbor.encodeAsync(
        documents.map((d) => d.serialize()),
      ),
    });
  }

  return documentQueryHandler;
}

module.exports = documentQueryHandlerFactory;

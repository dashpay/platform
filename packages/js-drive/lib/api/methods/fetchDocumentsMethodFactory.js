const InvalidParamsError = require('../InvalidParamsError');
const InvalidQueryError = require('../../stateView/document/errors/InvalidQueryError');

/**
 * @param {fetchDocuments} fetchDocuments
 * @returns {fetchDocumentsMethod}
 */
module.exports = function fetchDocumentsMethodFactory(fetchDocuments) {
  /**
   * @typedef {Promise} fetchDocumentsMethod
   * @param {{ contractId: string, type: string, options: Object }} params
   * @returns {Promise<Object[]>}
   */
  async function fetchDocumentsMethod(params) {
    if (!params.contractId) {
      throw new InvalidParamsError('Missing "contractId" param');
    }

    if (!params.type) {
      throw new InvalidParamsError('Missing "type" param');
    }

    try {
      const documents = await fetchDocuments(params.contractId, params.type, params.options);

      return documents.map(d => d.toJSON());
    } catch (error) {
      if (error instanceof InvalidQueryError) {
        throw new InvalidParamsError(error.message, error.getErrors());
      }

      throw error;
    }
  }

  return fetchDocumentsMethod;
};

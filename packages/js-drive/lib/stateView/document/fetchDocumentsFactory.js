/**
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(createSVDocumentRepository) {
  /**
   * Fetch original Documents by Contract ID and type
   *
   * @typedef {Promise} fetchDocuments
   * @param {string} contractId
   * @param {string} type
   * @param {Object} [options] options
   * @returns {Document[]}
   */
  async function fetchDocuments(contractId, type, options) {
    const svDocumentRepository = createSVDocumentRepository(contractId, type);
    const svDocuments = await svDocumentRepository.fetch(options);
    return svDocuments.map(svDocument => svDocument.getDocument());
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

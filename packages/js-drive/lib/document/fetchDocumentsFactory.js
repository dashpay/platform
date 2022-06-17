/**
 * @param {DocumentRepository} documentRepository
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(
  documentRepository,
) {
  /**
   * Fetch original Documents by Contract ID and type
   *
   * @typedef {Promise} fetchDocuments
   * @param {StorageResult<DataContract>} dataContractResult
   * @param {string} type
   * @param {Object} [options] options
   * @param {boolean} [options.useTransaction=false]
   * @returns {Promise<StorageResult<Document[]>>}
   */
  async function fetchDocuments(dataContractResult, type, options) {
    const dataContract = dataContractResult.getValue();
    const operations = dataContractResult.getOperations();

    const result = await documentRepository.find(
      dataContract,
      type,
      options,
    );

    result.addOperation(...operations);

    return result;
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

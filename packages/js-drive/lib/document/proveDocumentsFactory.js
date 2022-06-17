/**
 * @param {DocumentRepository} documentRepository
 * @returns {fetchDocuments}
 */
function proveDocumentsFactory(
  documentRepository,
) {
  /**
   *
   * @typedef {Promise} proveDocuments
   * @param {StorageResult<DataContract>} dataContractResult
   * @param {string} type
   * @param {Object} [options] options
   * @param {boolean} [options.useTransaction=false]
   * @returns {Promise<Document[]>}
   */
  async function proveDocuments(dataContractResult, type, options) {
    const dataContract = dataContractResult.getValue();
    const operations = dataContractResult.getOperations();

    const result = await documentRepository.prove(
      dataContract,
      type,
      options,
    );

    result.addOperation(...operations);

    return result;
  }

  return proveDocuments;
}

module.exports = proveDocumentsFactory;

/**
 * @param {DocumentRepository} documentRepository
 * @param {fetchDataContract} fetchDataContract
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(
  documentRepository,
  fetchDataContract,
) {
  /**
   * Fetch original Documents by Contract ID and type
   *
   * @typedef {Promise} fetchDocuments
   * @param {Buffer|Identifier} dataContractId
   * @param {string} type
   * @param {Object} [options] options
   * @param {boolean} [options.useTransaction=false]
   * @returns {Promise<Document[]>}
   */
  async function fetchDocuments(dataContractId, type, options) {
    const dataContractResult = await fetchDataContract(dataContractId, type);

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

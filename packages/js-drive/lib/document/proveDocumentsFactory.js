/**
 * @param {DocumentRepository} documentRepository
 * @param {fetchDataContract} fetchDataContract
 * @returns {fetchDocuments}
 */
function proveDocumentsFactory(
  documentRepository,
  fetchDataContract,
) {
  /**
   *
   * @typedef {Promise} proveDocuments
   * @param {Buffer|Identifier} dataContractId
   * @param {string} type
   * @param {Object} [options] options
   * @param {GroveDBTransaction} [options.transaction]
   * @returns {Promise<Document[]>}
   */
  async function proveDocuments(dataContractId, type, options) {
    const dataContractResult = await fetchDataContract(dataContractId);

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

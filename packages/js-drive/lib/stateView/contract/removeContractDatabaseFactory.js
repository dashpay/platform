/**
 *
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @returns {removeContractDatabase}
 */
function removeContractDatabaseFactory(createSVDocumentRepository) {
  /**
   * Remove collection for each documentType in Contract
   *
   * @typedef {Promise} removeContractDatabase
   * @param {SVContract} svContract
   * @returns {Promise<*[]>}
   */
  async function removeContractDatabase(svContract) {
    const documents = svContract.getContract().getDocuments();
    const contractId = svContract.getContractId();

    const promises = Object.keys(documents).map((documentType) => {
      const svDocumentRepository = createSVDocumentRepository(contractId, documentType);

      return svDocumentRepository.removeCollection();
    });

    return Promise.all(promises);
  }

  return removeContractDatabase;
}

module.exports = removeContractDatabaseFactory;

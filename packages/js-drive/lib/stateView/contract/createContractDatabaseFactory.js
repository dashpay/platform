/**
 *
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @returns {createContractDatabase}
 */
function createContractDatabaseFactory(createSVDocumentRepository) {
  /**
   * Create new collection for each documentType in Contract
   *
   * @typedef {Promise} createContractDatabase
   * @param {SVContract} svContract
   * @returns {Promise<*[]>}
   */
  async function createContractDatabase(svContract) {
    const documents = svContract.getContract().getDocuments();
    const contractId = svContract.getContractId();

    const promises = Object.keys(documents).map((documentType) => {
      const svDocumentRepository = createSVDocumentRepository(contractId, documentType);

      return svDocumentRepository.createCollection();
    });

    return Promise.all(promises);
  }

  return createContractDatabase;
}

module.exports = createContractDatabaseFactory;

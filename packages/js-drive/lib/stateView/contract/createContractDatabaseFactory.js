/**
 *
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @param {convertToMongoDbIndices} convertToMongoDbIndices
 * @returns {createContractDatabase}
 */
function createContractDatabaseFactory(createSVDocumentRepository, convertToMongoDbIndices) {
  /**
   * Create new collection for each documentType in Contract
   *
   * @typedef {Promise} createContractDatabase
   * @param {SVContract} svContract
   * @returns {Promise<*[]>}
   */
  async function createContractDatabase(svContract) {
    const dataContract = svContract.getDataContract();
    const documents = dataContract.getDocuments();
    const contractId = svContract.getId();

    const promises = Object.keys(documents).map((documentType) => {
      const documentSchema = dataContract.getDocumentSchema(documentType);
      let indices;
      if (documentSchema.indices) {
        indices = convertToMongoDbIndices(documentSchema.indices);
      }

      const svDocumentRepository = createSVDocumentRepository(contractId, documentType);

      return svDocumentRepository.createCollection(indices);
    });

    return Promise.all(promises);
  }

  return createContractDatabase;
}

module.exports = createContractDatabaseFactory;

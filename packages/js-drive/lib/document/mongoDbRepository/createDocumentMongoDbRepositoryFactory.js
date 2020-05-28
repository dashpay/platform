const DocumentMongoDbRepository = require('./DocumentMongoDbRepository');

/**
 * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
 * @param {validateQuery} validateQuery
 * @param {getDocumentDatabase} getDocumentDatabase
 * @returns {createDocumentMongoDbRepository}
 */
function createDocumentMongoDbRepositoryFactory(
  convertWhereToMongoDbQuery,
  validateQuery,
  getDocumentDatabase,
) {
  /**
   * Create DocumentMongoDbRepository
   *
   * @typedef {Promise} createDocumentMongoDbRepository
   * @param {string} dataContractId
   * @param {string} documentType
   * @returns {Promise<DocumentMongoDbRepository>}
   */
  async function createDocumentMongoDbRepository(dataContractId, documentType) {
    const documentsMongoDatabase = await getDocumentDatabase(dataContractId);

    return new DocumentMongoDbRepository(
      documentsMongoDatabase,
      convertWhereToMongoDbQuery,
      validateQuery,
      dataContractId,
      documentType,
    );
  }

  return createDocumentMongoDbRepository;
}

module.exports = createDocumentMongoDbRepositoryFactory;

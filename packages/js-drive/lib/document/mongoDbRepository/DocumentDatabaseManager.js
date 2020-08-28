class DocumentDatabaseManager {
  /**
   * @param {createDocumentMongoDbRepository} createDocumentRepository
   * @param {convertToMongoDbIndices} convertToMongoDbIndices
   * @param {getDocumentDatabase} getDocumentDatabase
   */
  constructor(
    createDocumentRepository,
    convertToMongoDbIndices,
    getDocumentDatabase,
  ) {
    this.createDocumentRepository = createDocumentRepository;
    this.convertToMongoDbIndices = convertToMongoDbIndices;
    this.getDocumentDatabase = getDocumentDatabase;
  }

  /**
   * Create a database for @dataContract documents
   *
   * @param {DataContract} dataContract
   * @returns {Promise<*[]>}
   */
  async create(dataContract) {
    const documentTypes = Object.keys(dataContract.getDocuments());

    const promises = documentTypes.map(async (documentType) => {
      const documentSchema = dataContract.getDocumentSchema(documentType);
      let indices;
      if (documentSchema.indices) {
        indices = this.convertToMongoDbIndices(documentSchema.indices);
      }

      const documentRepository = await this.createDocumentRepository(
        dataContract.getId(),
        documentType,
      );

      return documentRepository.createCollection(indices);
    });

    return Promise.all(promises);
  }

  /**
   * Drop @dataContract database
   *
   * @param {DataContract} dataContract
   * @returns {Promise<*[]>}
   */
  async drop(dataContract) {
    const db = await this.getDocumentDatabase(dataContract.getId());
    return db.dropDatabase();
  }
}

module.exports = DocumentDatabaseManager;

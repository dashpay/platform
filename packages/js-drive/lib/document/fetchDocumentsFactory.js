const InvalidQueryError = require('./errors/InvalidQueryError');
const InvalidDocumentTypeError = require('./query/errors/InvalidDocumentTypeError');
const InvalidContractIdError = require('./query/errors/InvalidContractIdError');
/**
 * @param {createDocumentMongoDbRepository} createDocumentRepository
 * @param {DataContractLevelDBRepository} dataContractRepository
 * @param {LRUCache} dataContractCache
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(
  createDocumentRepository,
  dataContractRepository,
  dataContractCache,
) {
  /**
   * Fetch original Documents by Contract ID and type
   *
   * @typedef {Promise} fetchDocuments
   * @param {string} contractId
   * @param {string} type
   * @param {Object} [options] options
   * @param {MongoDBTransaction} [dbTransaction]
   * @returns {Promise<Document[]>}
   */
  async function fetchDocuments(contractId, type, options, dbTransaction = undefined) {
    const documentRepository = createDocumentRepository(contractId, type);

    let dataContract = dataContractCache.get(contractId);

    if (!dataContract) {
      dataContract = await dataContractRepository.fetch(contractId);

      if (!dataContract) {
        const error = new InvalidContractIdError(contractId);

        throw new InvalidQueryError([error]);
      }

      dataContractCache.set(contractId, dataContract);
    }

    if (!dataContract.isDocumentDefined(type)) {
      const error = new InvalidDocumentTypeError(type);

      throw new InvalidQueryError([error]);
    }

    const documentSchema = dataContract.getDocumentSchema(type);

    return documentRepository.fetch(
      options,
      documentSchema,
      dbTransaction,
    );
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

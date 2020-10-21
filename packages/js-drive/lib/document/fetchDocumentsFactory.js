const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

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
   * @param {Buffer|Identifier} contractId
   * @param {string} type
   * @param {Object} [options] options
   * @param {MongoDBTransaction} [dbTransaction]
   * @returns {Promise<Document[]>}
   */
  async function fetchDocuments(contractId, type, options, dbTransaction = undefined) {
    let documentRepository;
    try {
      documentRepository = await createDocumentRepository(contractId, type);
    } catch (error) {
      if (error instanceof InvalidContractIdError) {
        throw new InvalidQueryError([error]);
      }

      throw error;
    }

    // eslint-disable-next-line no-param-reassign
    contractId = new Identifier(contractId);

    const contractIdString = contractId.toString();

    let dataContract = dataContractCache.get(contractIdString);

    if (!dataContract) {
      dataContract = await dataContractRepository.fetch(contractId);

      if (!dataContract) {
        const error = new InvalidContractIdError(contractId);

        throw new InvalidQueryError([error]);
      }

      dataContractCache.set(contractIdString, dataContract);
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

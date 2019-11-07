const InvalidQueryError = require('./errors/InvalidQueryError');
const InvalidDocumentTypeError = require('./query/errors/InvalidDocumentTypeError');
const InvalidContractIdError = require('./query/errors/InvalidContractIdError');
/**
 * @param {createSVDocumentMongoDbRepository} createSVDocumentRepository
 * @param {SVContractMongoDbRepository} svContractMongoDbRepository
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(
  createSVDocumentRepository,
  svContractMongoDbRepository,
) {
  /**
   * Fetch original Documents by Contract ID and type
   *
   * @typedef {Promise} fetchDocuments
   * @param {string} contractId
   * @param {string} type
   * @param {Object} [options] options
   * @param {MongoDBTransaction} [stateViewTransaction]
   * @returns {Document[]}
   */
  async function fetchDocuments(contractId, type, options, stateViewTransaction = undefined) {
    const svDocumentRepository = createSVDocumentRepository(contractId, type);

    const svContract = await svContractMongoDbRepository.find(contractId);
    if (!svContract) {
      const error = new InvalidContractIdError(contractId);

      throw new InvalidQueryError([error]);
    }

    const dataContract = svContract.getDataContract();
    if (!dataContract.isDocumentDefined(type)) {
      const error = new InvalidDocumentTypeError(type);

      throw new InvalidQueryError([error]);
    }

    const documentSchema = dataContract.getDocumentSchema(type);

    const svDocuments = await svDocumentRepository.fetch(
      options,
      documentSchema,
      stateViewTransaction,
    );

    return svDocuments.map(svDocument => svDocument.getDocument());
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const InvalidQueryError = require('./errors/InvalidQueryError');
const InvalidDocumentTypeError = require('./query/errors/InvalidDocumentTypeError');
const InvalidContractIdError = require('./query/errors/InvalidContractIdError');
/**
 * @param {DocumentRepository} documentRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {LRUCache} dataContractCache
 * @returns {fetchDocuments}
 */
function fetchDocumentsFactory(
  documentRepository,
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
   * @param {boolean} [options.useTransaction=false]
   * @returns {Promise<Document[]>}
   */
  async function fetchDocuments(contractId, type, options) {
    let contractIdIdentifier;
    try {
      contractIdIdentifier = new Identifier(contractId);
    } catch (e) {
      if (e instanceof IdentifierError) {
        const error = new InvalidContractIdError(contractId);

        throw new InvalidQueryError([error]);
      }

      throw e;
    }

    const contractIdString = contractIdIdentifier.toString();

    let dataContract = dataContractCache.get(contractIdString);

    let operations = [];

    if (!dataContract) {
      const dataContractResult = await dataContractRepository.fetch(contractIdIdentifier);

      if (dataContractResult.isNull()) {
        const error = new InvalidContractIdError(contractIdIdentifier);

        throw new InvalidQueryError([error]);
      }

      dataContract = dataContractResult.getValue();
      operations = dataContractResult.getOperations();

      dataContractCache.set(contractIdString, dataContract);
    }

    if (!dataContract.isDocumentDefined(type)) {
      const error = new InvalidDocumentTypeError(type);

      throw new InvalidQueryError([error]);
    }

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

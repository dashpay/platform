const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const InvalidQueryError = require('./errors/InvalidQueryError');

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
        throw new InvalidQueryError(`invalid data contract ID: ${e.message}`);
      }

      throw e;
    }

    const contractIdString = contractIdIdentifier.toString();

    let dataContract = dataContractCache.get(contractIdString);

    let operations = [];

    if (!dataContract) {
      const dataContractResult = await dataContractRepository.fetch(contractIdIdentifier);

      if (dataContractResult.isNull()) {
        throw new InvalidQueryError(`data contract ${contractIdIdentifier} not found`);
      }

      dataContract = dataContractResult.getValue();
      operations = dataContractResult.getOperations();

      dataContractCache.set(contractIdString, dataContract);
    }

    if (!dataContract.isDocumentDefined(type)) {
      throw new InvalidQueryError(`document type ${type} is not defined in the data contract`);
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

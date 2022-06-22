const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const InvalidQueryError = require('./errors/InvalidQueryError');
const StorageResult = require('../storage/StorageResult');

/**
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {LRUCache} dataContractCache
 * @returns {fetchDocuments}
 */
function fetchDataContractFactory(
  dataContractRepository,
  dataContractCache,
) {
  /**
   * Fetch Data Contract by Contract ID and type
   *
   * @typedef {Promise} fetchDataContract
   * @param {Buffer|Identifier} contractId
   * @param {string} type
   * @returns {Promise<StorageResult<DataContract>>}
   */
  async function fetchDataContract(contractId, type) {
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
    let dataContractResult;

    if (!dataContract) {
      dataContractResult = await dataContractRepository.fetch(contractIdIdentifier);

      if (dataContractResult.isNull()) {
        throw new InvalidQueryError(`data contract ${contractIdIdentifier} not found`);
      }

      dataContract = dataContractResult.getValue();

      dataContractCache.set(contractIdString, dataContract);
    } else {
      dataContractResult = new StorageResult(
        dataContract,
        [new ReadOperation(dataContract.toBuffer().length)],
      );
    }

    if (!dataContract.isDocumentDefined(type)) {
      throw new InvalidQueryError(`document type ${type} is not defined in the data contract`);
    }

    return dataContractResult;
  }

  return fetchDataContract;
}

module.exports = fetchDataContractFactory;

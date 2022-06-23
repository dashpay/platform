const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const InvalidQueryError = require('./errors/InvalidQueryError');
const StorageResult = require('../storage/StorageResult');
const DataContractCacheItem = require('../dataContract/DataContractCacheItem');

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
   * @returns {Promise<StorageResult<DataContract>>}
   */
  async function fetchDataContract(contractId) {
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

    /**
     * @type {DataContractCacheItem}
     */
    let cacheItem = dataContractCache.get(contractIdString);

    let dataContractResult;

    if (cacheItem) {
      dataContractResult = new StorageResult(
        cacheItem.getDataContract(),
        cacheItem.getOperations(),
      );
    } else {
      dataContractResult = await dataContractRepository.fetch(contractIdIdentifier);

      if (dataContractResult.isNull()) {
        throw new InvalidQueryError(`data contract ${contractIdIdentifier} not found`);
      }

      cacheItem = new DataContractCacheItem(
        dataContractResult.getValue(),
        dataContractResult.getOperations(),
      );

      dataContractCache.set(contractIdString, cacheItem);
    }

    return dataContractResult;
  }

  return fetchDataContract;
}

module.exports = fetchDataContractFactory;

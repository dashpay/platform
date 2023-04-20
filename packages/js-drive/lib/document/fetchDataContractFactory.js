const IdentifierError = require('@dashevo/dpp/lib/identifier/errors/IdentifierError');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const InvalidQueryError = require('./errors/InvalidQueryError');

/**
 * @param {DataContractStoreRepository} dataContractRepository
 * @returns {fetchDocuments}
 */
function fetchDataContractFactory(
  dataContractRepository,
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

    const dataContractResult = await dataContractRepository.fetch(contractIdIdentifier);

    if (dataContractResult.isNull()) {
      throw new InvalidQueryError(`data contract ${contractIdIdentifier} not found`);
    }

    return dataContractResult;
  }

  return fetchDataContract;
}

module.exports = fetchDataContractFactory;

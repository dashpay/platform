const InvalidQueryError = require('./errors/InvalidQueryError');

/**
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {WebAssembly.Instance} dppWasm
 * @returns {fetchDocuments}
 */
function fetchDataContractFactory(
  dataContractRepository,
  dppWasm,
) {
  /**
   * Fetch Data Contract by Contract ID and type
   *
   * @typedef {Promise} fetchDataContract
   * @param {Buffer|dppWasm.Identifier} contractId
   * @returns {Promise<StorageResult<DataContract>>}
   */
  async function fetchDataContract(contractId) {
    let contractIdIdentifier;
    try {
      contractIdIdentifier = new dppWasm.Identifier(contractId);
    } catch (e) {
      if (e instanceof dppWasm.IdentifierError) {
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

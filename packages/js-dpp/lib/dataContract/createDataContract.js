const DataContract = require('./DataContract');

/**
 * @typedef createDataContract
 * @param {RawDataContract} rawDataContract
 * @return {DataContract}
 */
function createDataContract(rawDataContract) {
  const dataContract = new DataContract(
    rawDataContract.contractId,
    rawDataContract.documents,
  );

  if (rawDataContract.$schema) {
    dataContract.setJsonMetaSchema(rawDataContract.$schema);
  }

  if (rawDataContract.version) {
    dataContract.setVersion(rawDataContract.version);
  }

  if (rawDataContract.definitions) {
    dataContract.setDefinitions(rawDataContract.definitions);
  }

  return dataContract;
}

module.exports = createDataContract;

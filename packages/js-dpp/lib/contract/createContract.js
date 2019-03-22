const Contract = require('./Contract');

/**
 * @typedef createContract
 * @param {RawContract} rawContract
 * @return {Contract}
 */
function createContract(rawContract) {
  const contract = new Contract(
    rawContract.name,
    rawContract.documents,
  );

  if (rawContract.$schema) {
    contract.setJsonMetaSchema(rawContract.$schema);
  }

  if (rawContract.version) {
    contract.setVersion(rawContract.version);
  }

  if (rawContract.definitions) {
    contract.setDefinitions(rawContract.definitions);
  }

  return contract;
}

module.exports = createContract;

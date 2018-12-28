const DapContract = require('./DapContract');

function createDapContract(rawDapContract) {
  const dapContract = new DapContract(
    rawDapContract.name,
    rawDapContract.dapObjectsDefinition,
  );

  if (rawDapContract.$schema) {
    dapContract.setJsonMetaSchema(rawDapContract.$schema);
  }

  if (rawDapContract.version) {
    dapContract.setVersion(rawDapContract.version);
  }

  if (rawDapContract.definitions) {
    dapContract.setDefinitions(rawDapContract.definitions);
  }

  return dapContract;
}

module.exports = createDapContract;

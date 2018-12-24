const DapContract = require('./DapContract');

function createDapContract(object) {
  const dapContract = new DapContract(object.name, object.dapObjectsDefinition);

  if (object.$schema) {
    dapContract.setJsonMetaSchema(object.$schema);
  }

  if (object.version) {
    dapContract.setVersion(object.version);
  }

  if (object.definitions) {
    dapContract.setDefinitions(object.definitions);
  }

  return dapContract;
}

module.exports = createDapContract;

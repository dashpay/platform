const createDIContainer = require('../createDIContainer');

async function createTestDIContainer(mongoDB, dashCore = undefined) {
  const documentMongoDBUrl = `mongodb://127.0.0.1:${mongoDB.options.getMongoPort()}`
    + `/?replicaSet=${mongoDB.options.options.replicaSetName}`;

  let coreOptions = {};
  if (dashCore) {
    coreOptions = {
      CORE_JSON_RPC_HOST: '127.0.0.1',
      CORE_JSON_RPC_PORT: dashCore.options.getRpcPort(),
      CORE_JSON_RPC_USERNAME: dashCore.options.getRpcUser(),
      CORE_JSON_RPC_PASSWORD: dashCore.options.getRpcPassword(),
    };
  }

  return createDIContainer({
    ...process.env,
    DOCUMENT_MONGODB_URL: documentMongoDBUrl,
    COMMON_MERK_DB_FILE: `./db/common-merkdb-test/${Math.random()}`,
    IDENTITY_LEVEL_DB_FILE: './db/identity-test',
    DATA_CONTRACTS_MERK_DB_FILE: `./db/data-contracts-merkdb-test/${Math.random()}`,
    IDENTITIES_MERK_DB_FILE: `./db/identities-merkdb-test/${Math.random()}`,
    ...coreOptions,
  });
}

module.exports = createTestDIContainer;

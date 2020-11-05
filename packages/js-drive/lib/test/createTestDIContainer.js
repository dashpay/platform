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
    COMMON_STORE_MERK_DB_FILE: `./db/common-merkdb-test/${Math.random()}`,
    DATA_CONTRACTS_STORE_MERK_DB_FILE: `./db/data-contracts-merkdb-test/${Math.random()}`,
    DOCUMENTS_STORE_MERK_DB_FILE: `./db/documents-merkdb-test/${Math.random()}`,
    IDENTITIES_STORE_MERK_DB_FILE: `./db/identities-merkdb-test/${Math.random()}`,
    PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE: `./db/public-key-to-identity-id-merkdb-test/${Math.random()}`,
    ...coreOptions,
  });
}

module.exports = createTestDIContainer;

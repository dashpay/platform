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

  const container = createDIContainer({
    ...process.env,
    DOCUMENT_MONGODB_URL: documentMongoDBUrl,
    COMMON_STORE_MERK_DB_FILE: './db/common-merkdb-test',
    PREVIOUS_COMMON_STORE_MERK_DB_FILE: './db/common-merkdb-previous-test',
    DATA_CONTRACTS_STORE_MERK_DB_FILE: './db/data-contracts-merkdb-test',
    PREVIOUS_DATA_CONTRACTS_STORE_MERK_DB_FILE: './db/data-contracts-merkdb-previous-test',
    DOCUMENTS_STORE_MERK_DB_FILE: './db/documents-merkdb-test',
    PREVIOUS_DOCUMENTS_STORE_MERK_DB_FILE: './db/documents-merkdb-previous-test',
    IDENTITIES_STORE_MERK_DB_FILE: './db/identities-merkdb-test',
    PREVIOUS_IDENTITIES_STORE_MERK_DB_FILE: './db/identities-merkdb-previous-test',
    PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE: './db/public-key-to-identity-id-merkdb-test',
    PREVIOUS_PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE: './db/public-key-to-identity-id-merkdb-previous-test',
    EXTERNAL_STORE_LEVEL_DB_FILE: './db/external-leveldb-test',
    PREVIOUS_EXTERNAL_STORE_LEVEL_DB_FILE: './db/external-leveldb-previous-test',
    ...coreOptions,
  });

  const dpp = container.resolve('dpp');
  const transactionalDpp = container.resolve('transactionalDpp');

  await dpp.initialize();
  await transactionalDpp.initialize();

  return container;
}

module.exports = createTestDIContainer;

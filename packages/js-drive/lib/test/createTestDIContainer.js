const createDIContainer = require('../createDIContainer');

async function createTestDIContainer(mongoDB, dashCore = undefined) {
  const documentMongoDBUrl = `mongodb://127.0.0.1:${mongoDB.options.getMongoPort()}`
    + `/?replicaSet=${mongoDB.options.options.replicaSetName}`;

  let coreOptions = {};
  if (dashCore) {
    coreOptions = {
      CORE_JSON_RPC_HOST: dashCore.getIp(),
      CORE_JSON_RPC_PORT: dashCore.options.getRpcPort(),
      CORE_JSON_RPC_USERNAME: dashCore.options.getRpcUser(),
      CORE_JSON_RPC_PASSWORD: dashCore.options.getRpcPassword(),
    };
  }

  return createDIContainer({
    ...process.env,
    DOCUMENT_MONGODB_URL: documentMongoDBUrl,
    BLOCKCHAIN_STATE_LEVEL_DB_FILE: './db/blockchain-state-test',
    IDENTITY_LEVEL_DB_FILE: './db/identity-test',
    DATA_CONTRACT_LEVEL_DB_FILE: './db/data-contract-test',
    ...coreOptions,
  });
}

module.exports = createTestDIContainer;

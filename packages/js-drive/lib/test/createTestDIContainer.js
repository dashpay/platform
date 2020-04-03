const createDIContainer = require('../createDIContainer');

async function createTestDIContainer(mongoDB) {
  const documentMongoDBUrl = `mongodb://127.0.0.1:${mongoDB.options.getMongoPort()}`
    + `/?replicaSet=${mongoDB.options.options.replicaSetName}`;

  return createDIContainer({
    ...process.env,
    DOCUMENT_MONGODB_URL: documentMongoDBUrl,
    BLOCKCHAIN_STATE_LEVEL_DB_FILE: './db/blockchain-state-test',
    IDENTITY_LEVEL_DB_FILE: './db/identity-test',
    DATA_CONTRACT_LEVEL_DB_FILE: './db/data-contract-test',
  });
}

module.exports = createTestDIContainer;

const createDIContainer = require('../createDIContainer');

async function createTestDIContainer(dashCore = undefined) {
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
    GROVEDB_LATEST_FILE: './db/latest_state_test',
    EXTERNAL_STORE_LEVEL_DB_FILE: './db/external_leveldb_test',
    SIGNED_EXTERNAL_STORE_LEVEL_DB_FILE: './db/external_leveldb_signed',
    ...coreOptions,
  });

  const dpp = container.resolve('dpp');
  const transactionalDpp = container.resolve('transactionalDpp');

  await dpp.initialize();
  await transactionalDpp.initialize();

  return container;
}

module.exports = createTestDIContainer;

require('dotenv-expand')(require('dotenv-safe').config());

const createServer = require('abci');

const createDIContainer = require('../lib/createDIContainer');

const errorHandler = require('../lib/errorHandler');

(async function main() {
  const container = await createDIContainer(process.env);

  const logger = container.resolve('logger');

  logger.info('Connecting to MongoDB');
  const waitReplicaSetInitialize = container.resolve('waitReplicaSetInitialize');
  await waitReplicaSetInitialize((retry, maxRetries) => {
    logger.info(
      `waiting for replica set to be initialized ${retry}/${maxRetries}...`,
    );
  });

  logger.info('Connecting to Core');
  const checkCoreSyncFinished = container.resolve('checkCoreSyncFinished');
  await checkCoreSyncFinished((currentBlockHeight, currentHeaderNumber) => {
    logger.info(
      `waiting for Core to finish sync ${currentBlockHeight}/${currentHeaderNumber}...`,
    );
  });

  const server = createServer(
    container.resolve('abciHandlers'),
  );

  server.listen(
    container.resolve('abciPort'),
    container.resolve('abciHost'),
  );

  logger.info(`Drive ABCI is listening on port ${container.resolve('abciPort')}`);
}());

process
  .on('unhandledRejection', errorHandler)
  .on('uncaughtException', errorHandler);

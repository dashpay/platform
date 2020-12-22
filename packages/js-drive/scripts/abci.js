require('dotenv-expand')(require('dotenv-safe').config());

const createServer = require('@dashevo/abci');
const { onShutdown } = require('node-graceful-shutdown');

const createDIContainer = require('../lib/createDIContainer');

(async function main() {
  const container = await createDIContainer(process.env);
  const logger = container.resolve('logger');
  const errorHandler = container.resolve('errorHandler');

  /**
   * Ensure graceful shutdown
   */

  process
    .on('unhandledRejection', errorHandler)
    .on('uncaughtException', errorHandler);

  onShutdown('abci', async () => {
    await container.dispose();
  });

  /**
   * Make sure MongoDB is running
   */

  logger.info('Connecting to MongoDB');
  const waitReplicaSetInitialize = container.resolve('waitReplicaSetInitialize');
  await waitReplicaSetInitialize((retry, maxRetries) => {
    logger.info(
      `waiting for replica set to be initialized ${retry}/${maxRetries}...`,
    );
  });

  logger.info('Connecting to Core');

  const detectStandaloneRegtestMode = container.resolve('detectStandaloneRegtestMode');
  const isStandaloneRegtestMode = await detectStandaloneRegtestMode();

  /**
   * Make sure Core is synced
   */

  if (!isStandaloneRegtestMode) {
    const waitForCoreSync = container.resolve('waitForCoreSync');
    await waitForCoreSync((currentBlockHeight, currentHeaderNumber) => {
      logger.info(
        `waiting for Core to finish sync ${currentBlockHeight}/${currentHeaderNumber}...`,
      );
    });
  }

  /**
   * Connect to Core ZMQ socket
   */

  const coreZMQClient = container.resolve('coreZMQClient');
  const coreZMQConnectionRetries = container.resolve('coreZMQConnectionRetries');

  coreZMQClient.on('connect', () => {
    logger.trace('Connected to Core ZMQ socket');
  });

  coreZMQClient.on('disconnect', () => {
    logger.trace('Disconnected from Core ZMQ socket');
  });

  coreZMQClient.on('connect:max_retry_exceeded', async () => {
    const error = new Error('Can\'t connect to Core ZMQ');

    await errorHandler(error);
  });

  try {
    await coreZMQClient.connect({
      maxRetries: coreZMQConnectionRetries,
    });
  } catch (e) {
    const error = new Error(`Can't connect to Core ZMQ socket: ${e.message}`);

    await errorHandler(error);
  }

  if (!isStandaloneRegtestMode) {
    logger.info('Obtaining the latest Core ChainLock...');
    const waitForCoreChainLockSync = container.resolve('waitForCoreChainLockSync');
    await waitForCoreChainLockSync();
  } else {
    logger.info('Obtaining the latest core block for chain lock sync fallback...');
    const waitForCoreChainLockSyncFallback = container.resolve('waitForCoreChainLockSyncFallback');
    await waitForCoreChainLockSyncFallback();
  }

  logger.info('Waining for initial Core ChainLocked height...');
  const waitForChainLockedHeight = container.resolve('waitForChainLockedHeight');
  const initialCoreChainLockedHeight = container.resolve('initialCoreChainLockedHeight');
  await waitForChainLockedHeight(initialCoreChainLockedHeight);

  const server = createServer(
    container.resolve('abciHandlers'),
  );

  server.on('error', async (e) => {
    await errorHandler(e);
  });

  server.listen(
    container.resolve('abciPort'),
    container.resolve('abciHost'),
  );

  logger.info(`Drive ABCI is listening on port ${container.resolve('abciPort')}`);
}());

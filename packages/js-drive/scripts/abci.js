require('dotenv-expand')(require('dotenv-safe').config());

const graceful = require('node-graceful');

const chalk = require('chalk');

const ZMQClient = require('../lib/core/ZmqClient');

const createDIContainer = require('../lib/createDIContainer');

const { version: driveVersion } = require('../package.json');

const banner = '\n ____       ______      ____        __  __                 ____       ____        ______      __  __     ____      \n'
+ '/\\  _`\\    /\\  _  \\    /\\  _`\\     /\\ \\/\\ \\               /\\  _`\\    /\\  _`\\     /\\__  _\\    /\\ \\/\\ \\   /\\  _`\\    \n'
+ '\\ \\ \\/\\ \\  \\ \\ \\L\\ \\   \\ \\,\\L\\_\\   \\ \\ \\_\\ \\              \\ \\ \\/\\ \\  \\ \\ \\L\\ \\   \\/_/\\ \\/    \\ \\ \\ \\ \\  \\ \\ \\L\\_\\  \n'
+ ' \\ \\ \\ \\ \\  \\ \\  __ \\   \\/_\\__ \\    \\ \\  _  \\              \\ \\ \\ \\ \\  \\ \\ ,  /      \\ \\ \\     \\ \\ \\ \\ \\  \\ \\  _\\L  \n'
+ '  \\ \\ \\_\\ \\  \\ \\ \\/\\ \\    /\\ \\L\\ \\   \\ \\ \\ \\ \\              \\ \\ \\_\\ \\  \\ \\ \\\\ \\      \\_\\ \\__   \\ \\ \\_/ \\  \\ \\ \\L\\ \\\n'
+ '   \\ \\____/   \\ \\_\\ \\_\\   \\ `\\____\\   \\ \\_\\ \\_\\              \\ \\____/   \\ \\_\\ \\_\\    /\\_____\\   \\ `\\___/   \\ \\____/\n'
+ '    \\/___/     \\/_/\\/_/    \\/_____/    \\/_/\\/_/               \\/___/     \\/_/\\/ /    \\/_____/    `\\/__/     \\/___/\n\n\n';

// eslint-disable-next-line no-console
console.log(chalk.hex('#008de4')(banner));

(async function main() {
  const container = createDIContainer(process.env);
  const logger = container.resolve('logger');
  const dpp = container.resolve('dpp');
  const transactionalDpp = container.resolve('transactionalDpp');
  const errorHandler = container.resolve('errorHandler');
  const protocolVersion = container.resolve('protocolVersion');
  const closeAbciServer = container.resolve('closeAbciServer');

  logger.info(`Starting Drive ABCI application v${driveVersion} (protocol v${protocolVersion})`);

  /**
   * Ensure graceful shutdown
   */

  process
    .on('unhandledRejection', errorHandler)
    .on('uncaughtException', errorHandler);

  graceful.DEADLY_SIGNALS.push('SIGQUIT');

  graceful.on('exit', async (signal) => {
    logger.info({ signal }, `Received ${signal}. Stopping Drive ABCI application...`);

    await closeAbciServer();

    await container.dispose();
  });

  /**
   * Initialize DPP
   */

  await dpp.initialize();
  await transactionalDpp.initialize();

  /**
   * Make sure MongoDB is running
   */

  logger.info('Connecting to MongoDB...');

  const waitReplicaSetInitialize = container.resolve('waitReplicaSetInitialize');
  await waitReplicaSetInitialize((retry, maxRetries) => {
    logger.info(
      `waiting for replica set to be initialized ${retry}/${maxRetries}...`,
    );
  });

  /**
   * Make sure Core is synced
   */

  const network = container.resolve('network');

  logger.info(`Connecting to Core in ${network} network...`);

  const waitForCoreSync = container.resolve('waitForCoreSync');
  await waitForCoreSync((currentBlockHeight, currentHeaderNumber) => {
    let message = `waiting for core to finish sync ${currentBlockHeight}/${currentHeaderNumber}...`;

    if (currentBlockHeight === 0 && currentHeaderNumber === 0) {
      message = 'waiting for core to connect to peers...';
    }

    logger.info(message);
  });

  /**
   * Connect to Core ZMQ socket
   */

  const coreZMQClient = container.resolve('coreZMQClient');

  coreZMQClient.on(ZMQClient.events.CONNECTED, () => {
    logger.debug('Connected to core ZMQ socket');
  });

  coreZMQClient.on(ZMQClient.events.DISCONNECTED, () => {
    logger.debug('Disconnected from core ZMQ socket');
  });

  coreZMQClient.on(ZMQClient.events.MAX_RETRIES_REACHED, async () => {
    const error = new Error('Can\'t connect to core ZMQ');

    await errorHandler(error);
  });

  try {
    await coreZMQClient.start();
  } catch (e) {
    const error = new Error(`Can't connect to core ZMQ socket: ${e.message}`);

    await errorHandler(error);
  }

  /**
   * Obtain chain lock
   */

  logger.info('Obtaining the latest chain lock...');

  const waitForCoreChainLockSync = container.resolve('waitForCoreChainLockSync');
  await waitForCoreChainLockSync();

  /**
   * Wait for initial core chain locked height
   */
  const initialCoreChainLockedHeight = container.resolve('initialCoreChainLockedHeight');

  logger.info(`Waiting for initial core chain locked height #${initialCoreChainLockedHeight}...`);

  const waitForChainLockedHeight = container.resolve('waitForChainLockedHeight');
  await waitForChainLockedHeight(initialCoreChainLockedHeight);

  /**
   * Start ABCI server
   */

  const abciServer = container.resolve('abciServer');

  abciServer.on('connection', (socket) => {
    logger.debug(
      {
        abciConnectionId: socket.connection.id,
      },
      `Accepted new ABCI connection #${socket.connection.id} from ${socket.remoteAddress}:${socket.remotePort}`,
    );

    socket.on('error', (e) => {
      logger.error(
        {
          err: e,
          abciConnectionId: socket.connection.id,
        },
        `ABCI connection #${socket.connection.id} error: ${e.message}`,
      );
    });

    socket.once('close', (hasError) => {
      let message = `ABCI connection #${socket.connection.id} is closed`;
      if (hasError) {
        message += ' with error';
      }

      logger.debug(
        {
          abciConnectionId: socket.connection.id,
        },
        message,
      );
    });
  });

  abciServer.once('close', () => {
    logger.info('ABCI server and all connections are closed');
  });

  abciServer.on('error', async (e) => {
    await errorHandler(e);
  });

  abciServer.on('listening', () => {
    logger.info(`ABCI server is waiting for connection on port ${container.resolve('abciPort')}`);
  });

  abciServer.listen(
    container.resolve('abciPort'),
    container.resolve('abciHost'),
  );
}());

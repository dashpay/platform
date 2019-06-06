// Entry point for DAPI.
const dotenv = require('dotenv');
const grpc = require('grpc');

// Load config from .env
dotenv.config();

const config = require('../lib/config');
const { validateConfig } = require('../lib/config/validator');
const log = require('../lib/log');
const rpcServer = require('../lib/rpcServer/server');
const QuorumService = require('../lib/services/quorum');
const ZmqClient = require('../lib/externalApis/dashcore/ZmqClient');
const DriveAdapter = require('../lib/externalApis/driveAdapter');
const insightAPI = require('../lib/externalApis/insight');
const dashCoreRpcClient = require('../lib/externalApis/dashcore/rpc');
const userIndex = require('../lib/services/userIndex');

const createServer = require('../lib/grpcServer/createServer');

async function main() {
  /* Application start */
  const configValidationResult = validateConfig(config);
  if (!configValidationResult.isValid) {
    configValidationResult.validationErrors.forEach(log.error);
    log.log('Aborting DAPI startup due to config validation errors');
    process.exit();
  }

  // Subscribe to events from dashcore
  const dashCoreZmqClient = new ZmqClient(config.dashcore.zmq.host, config.dashcore.zmq.port);
  // Bind logs on ZMQ connection events
  dashCoreZmqClient.on(ZmqClient.events.DISCONNECTED, log.warn);
  dashCoreZmqClient.on(ZmqClient.events.CONNECTION_DELAY, log.warn);
  dashCoreZmqClient.on(ZmqClient.events.MONITOR_ERROR, log.warn);
  // Wait until zmq connection is established
  log.info(`Connecting to dashcore ZMQ on ${dashCoreZmqClient.connectionString}`);
  await dashCoreZmqClient.start();
  log.info('Connection to ZMQ established.');

  log.info('Staring quorum service');
  const quorumService = new QuorumService({
    dashCoreRpcClient,
    dashCoreZmqClient,
    log,
  });
  quorumService.start(dashCoreZmqClient);
  log.info('Quorum service started');

  log.info('Connecting to Drive');
  const driveAPI = new DriveAdapter({
    host: config.drive.host,
    port: config.drive.port,
  });

  log.info('Starting username index service');
  userIndex.start({
    dashCoreZmqClient,
    dashCoreRpcClient,
    log,
  });
  log.info('Username index service started');

  // Start JSON RPC server
  log.info('Starting JSON RPC server');
  rpcServer.start({
    port: config.rpcServer.port,
    networkType: config.network,
    insightAPI,
    dashcoreAPI: dashCoreRpcClient,
    driveAPI,
    userIndex,
    log,
  });
  log.info(`JSON RPC server is listening on port ${config.rpcServer.port}`);

  // Start GRPC server
  log.info('Starting GRPC server');

  const grpcServer = createServer('Core', { });

  grpcServer.bind(
    `0.0.0.0:${config.txFilterStream.grpcServer.port}`,
    grpc.ServerCredentials.createInsecure(),
  );

  grpcServer.start();

  log.info(`GRPC RPC server is listening on port ${config.txFilterStream.grpcServer.port}`);

  // Display message that everything is ok
  log.info(`Insight uri is ${config.insightUri}`);
  log.info(`DAPI Core process is up and running in ${config.livenet ? 'livenet' : 'testnet'} mode`);
  log.info(`Network is ${config.network}`);
}

main().catch((e) => {
  log.error(e.stack);
  process.exit();
});

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});

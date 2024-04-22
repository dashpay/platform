// Entry point for DAPI.
const dotenv = require('dotenv');
const grpc = require('@grpc/grpc-js');
// const loadBLS = require('@dashevo/bls');

const {
  server: {
    createServer,
  },
} = require('@dashevo/grpc-common');

const {
  getCoreDefinition,
  getPlatformDefinition,
} = require('@dashevo/dapi-grpc');

const { default: loadWasmDpp, DashPlatformProtocol } = require('@dashevo/wasm-dpp');

const { client: RpcClient } = require('jayson/promise');

// Load config from .env
dotenv.config();

const WsClient = require('../lib/externalApis/tenderdash/WsClient');

const config = require('../lib/config');
const { validateConfig } = require('../lib/config/validator');
const logger = require('../lib/logger');
const rpcServer = require('../lib/rpcServer/server');
const DriveClient = require('../lib/externalApis/drive/DriveClient');
const dashCoreRpcClient = require('../lib/externalApis/dashcore/rpc');
const BlockchainListener = require('../lib/externalApis/tenderdash/BlockchainListener');
// const DriveStateRepository = require('../lib/dpp/DriveStateRepository');

const coreHandlersFactory = require(
  '../lib/grpcServer/handlers/core/coreHandlersFactory',
);
const platformHandlersFactory = require(
  '../lib/grpcServer/handlers/platform/platformHandlersFactory',
);

async function main() {
  await loadWasmDpp();
  // const blsSignatures = await loadBLS();

  /* Application start */
  const configValidationResult = validateConfig(config);
  if (!configValidationResult.isValid) {
    configValidationResult.validationErrors.forEach(logger.fatal.bind(logger));
    logger.log('Aborting DAPI startup due to config validation errors');
    process.exit();
  }

  const isProductionEnvironment = process.env.NODE_ENV === 'production';

  logger.info('Connecting to Drive');
  const driveClient = new DriveClient({
    host: config.tendermintCore.host,
    port: config.tendermintCore.port,
  });

  const rpcClient = RpcClient.http({
    host: config.tendermintCore.host,
    port: config.tendermintCore.port,
  });

  const tenderDashWsClient = new WsClient({
    host: config.tendermintCore.host,
    port: config.tendermintCore.port,
  });

  const tenderdashLogger = logger.child({
    process: 'TenderdashWs',
  });

  tenderdashLogger.info(`Connecting to Tenderdash on ${config.tendermintCore.host}:${config.tendermintCore.port}`);

  tenderDashWsClient.on('connect', () => {
    tenderdashLogger.info('Connection to Tenderdash established.');
  });

  tenderDashWsClient.on('connect:retry', ({ interval }) => {
    tenderdashLogger.info(`Reconnect to Tenderdash in ${interval} ms`);
  });

  tenderDashWsClient.on('connect:max_retry_exceeded', ({ maxRetries }) => {
    tenderdashLogger.info(`Connection retry limit ${maxRetries} is reached`);
  });

  tenderDashWsClient.on('error', ({ error }) => {
    tenderdashLogger.error(`Tenderdash connection error: ${error.message}`);
  });

  tenderDashWsClient.on('close', ({ error }) => {
    tenderdashLogger.warn(`Connection closed: ${error.code}`);
  });

  tenderDashWsClient.on('disconnect', () => {
    tenderdashLogger.fatal('Disconnected from Tenderdash... exiting');

    process.exit(1);
  });

  await tenderDashWsClient.connect();

  const blockchainListener = new BlockchainListener(tenderDashWsClient);
  blockchainListener.start();

  // Start JSON RPC server
  logger.info('Starting JSON RPC server');
  rpcServer.start({
    port: config.rpcServer.port,
    networkType: config.network,
    dashcoreAPI: dashCoreRpcClient,
    logger,
  });
  logger.info(`JSON RPC server is listening on port ${config.rpcServer.port}`);

  const dpp = new DashPlatformProtocol(null, 1);

  // Start GRPC server
  logger.info('Starting GRPC server');

  const coreHandlers = coreHandlersFactory(
    dashCoreRpcClient,
    isProductionEnvironment,
  );
  const platformHandlers = platformHandlersFactory(
    rpcClient,
    blockchainListener,
    driveClient,
    dpp,
    isProductionEnvironment,
  );

  const grpcApiServer = createServer(getCoreDefinition(0), coreHandlers);

  grpcApiServer.addService(getPlatformDefinition(0).service, platformHandlers);

  grpcApiServer.bindAsync(
    `0.0.0.0:${config.grpcServer.port}`,
    grpc.ServerCredentials.createInsecure(),
    () => {
      grpcApiServer.start();
    },
  );

  logger.info(`GRPC API RPC server is listening on port ${config.grpcServer.port}`);

  // Display message that everything is ok
  logger.info(`DAPI Core process is up and running in ${config.livenet ? 'livenet' : 'testnet'} mode`);
  logger.info(`Network is ${config.network}`);
}

main().catch((e) => {
  logger.error(e.stack);

  process.exit(1);
});

process.on('unhandledRejection', (e) => {
  logger.error(e);

  process.exit(1);
});

// break on ^C
process.on('SIGINT', () => {
  logger.info('Received SIGINT. Exiting...');

  process.exit();
});

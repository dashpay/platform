// Entry point for DAPI.
const dotenv = require('dotenv');
const grpc = require('grpc');

const {
  server: {
    createServer,
  },
} = require('@dashevo/grpc-common');

const {
  getCoreDefinition,
  getPlatformDefinition,
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');

const { client: RpcClient } = require('jayson/promise');

const WsClient = require('../lib/externalApis/tenderdash/WsClient');

// Load config from .env
dotenv.config();

const config = require('../lib/config');
const { validateConfig } = require('../lib/config/validator');
const log = require('../lib/log');
const rpcServer = require('../lib/rpcServer/server');
const DriveClient = require('../lib/externalApis/drive/DriveClient');
const insightAPI = require('../lib/externalApis/insight');
const dashCoreRpcClient = require('../lib/externalApis/dashcore/rpc');
const BlockchainListener = require('../lib/externalApis/tenderdash/BlockchainListener');
const DriveStateRepository = require('../lib/dpp/DriveStateRepository');

const coreHandlersFactory = require(
  '../lib/grpcServer/handlers/core/coreHandlersFactory',
);
const platformHandlersFactory = require(
  '../lib/grpcServer/handlers/platform/platformHandlersFactory',
);

async function main() {
  /* Application start */
  const configValidationResult = validateConfig(config);
  if (!configValidationResult.isValid) {
    configValidationResult.validationErrors.forEach(log.error);
    log.log('Aborting DAPI startup due to config validation errors');
    process.exit();
  }

  const isProductionEnvironment = process.env.NODE_ENV === 'production';

  log.info('Connecting to Drive');
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

  const dppForParsingContracts = new DashPlatformProtocol();
  const driveStateRepository = new DriveStateRepository(driveClient, dppForParsingContracts);

  log.info(`Connecting to Tenderdash on ${config.tendermintCore.host}:${config.tendermintCore.port}`);

  tenderDashWsClient.on('error', (e) => {
    log.error('Tenderdash connection error', e);

    process.exit(1);
  });

  await tenderDashWsClient.connect();

  const blockchainListener = new BlockchainListener(tenderDashWsClient);
  blockchainListener.start();

  log.info('Connection to Tenderdash established.');

  // Start JSON RPC server
  log.info('Starting JSON RPC server');
  rpcServer.start({
    port: config.rpcServer.port,
    networkType: config.network,
    dashcoreAPI: dashCoreRpcClient,
    log,
  });
  log.info(`JSON RPC server is listening on port ${config.rpcServer.port}`);

  const dpp = new DashPlatformProtocol({
    stateRepository: driveStateRepository,
  });

  // Start GRPC server
  log.info('Starting GRPC server');

  const coreHandlers = coreHandlersFactory(
    insightAPI,
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

  grpcApiServer.bind(
    `0.0.0.0:${config.grpcServer.port}`,
    grpc.ServerCredentials.createInsecure(),
  );

  grpcApiServer.start();

  log.info(`GRPC API RPC server is listening on port ${config.grpcServer.port}`);

  // Display message that everything is ok
  log.info(`Insight uri is ${config.insightUri}`);
  log.info(`DAPI Core process is up and running in ${config.livenet ? 'livenet' : 'testnet'} mode`);
  log.info(`Network is ${config.network}`);
}

main().catch((e) => {
  log.error(e.stack);

  process.exit(1);
});

process.on('unhandledRejection', (e) => {
  log.error(e);

  process.exit(1);
});

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});

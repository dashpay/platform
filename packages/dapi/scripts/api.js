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

const { client: RpcClient } = require('jayson/promise');

// Load config from .env
dotenv.config();

const config = require('../lib/config');
const { validateConfig } = require('../lib/config/validator');
const log = require('../lib/log');
const rpcServer = require('../lib/rpcServer/server');
const DriveStateRepository = require('../lib/externalApis/drive/DriveStateRepository');
const insightAPI = require('../lib/externalApis/insight');
const dashCoreRpcClient = require('../lib/externalApis/dashcore/rpc');

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

  log.info('Connecting to Drive');
  const driveStateRepository = new DriveStateRepository({
    host: config.tendermintCore.host,
    port: config.tendermintCore.port,
  });

  const rpcClient = RpcClient.http({
    host: config.tendermintCore.host,
    port: config.tendermintCore.port,
  });

  // Start JSON RPC server
  log.info('Starting JSON RPC server');
  rpcServer.start({
    port: config.rpcServer.port,
    networkType: config.network,
    dashcoreAPI: dashCoreRpcClient,
    log,
  });
  log.info(`JSON RPC server is listening on port ${config.rpcServer.port}`);

  // Start GRPC server
  log.info('Starting GRPC server');

  const coreHandlers = coreHandlersFactory(insightAPI);
  const platformHandlers = platformHandlersFactory(rpcClient, driveStateRepository);

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
  process.exit();
});

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});

const dotenv = require('dotenv');
const grpc = require('@grpc/grpc-js');

const {
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
  server: {
    createServer,
    jsonToProtobufHandlerWrapper,
    error: {
      wrapInErrorHandlerFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    MasternodeListRequest,
    TransactionsWithProofsRequest,
    BlockHeadersWithChainLocksRequest,
    pbjs: {
      MasternodeListRequest: PBJSMasternodeListRequest,
      MasternodeListResponse: PBJSMasternodeListResponse,
      TransactionsWithProofsRequest: PBJSTransactionsWithProofsRequest,
      TransactionsWithProofsResponse: PBJSTransactionsWithProofsResponse,
      BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
      BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
    },
  },
  getCoreDefinition,
} = require('@dashevo/dapi-grpc');

// Load config from .env
dotenv.config();

const ChainDataProvider = require('../lib/chainDataProvider/ChainDataProvider');

const config = require('../lib/config');
const { validateConfig } = require('../lib/config/validator');
const logger = require('../lib/logger');

const BlockHeadersCache = require('../lib/chainDataProvider/BlockHeadersCache');
const ZmqClient = require('../lib/externalApis/dashcore/ZmqClient');
const dashCoreRpcClient = require('../lib/externalApis/dashcore/rpc');

const BloomFilterEmitterCollection = require('../lib/bloomFilter/emitter/BloomFilterEmitterCollection');

const testTransactionAgainstFilterCollectionFactory = require('../lib/transactionsFilter/testRawTransactionAgainstFilterCollectionFactory');
const emitBlockEventToFilterCollectionFactory = require('../lib/transactionsFilter/emitBlockEventToFilterCollectionFactory');
const testTransactionsAgainstFilter = require('../lib/transactionsFilter/testTransactionAgainstFilter');
const emitInstantLockToFilterCollectionFactory = require('../lib/transactionsFilter/emitInstantLockToFilterCollectionFactory');
const subscribeToTransactionsWithProofsHandlerFactory = require('../lib/grpcServer/handlers/tx-filter-stream/subscribeToTransactionsWithProofsHandlerFactory');
const subscribeToBlockHeadersWithChainLocksHandlerFactory = require('../lib/grpcServer/handlers/blockheaders-stream/subscribeToBlockHeadersWithChainLocksHandlerFactory');
const getHistoricalBlockHeadersIteratorFactory = require('../lib/grpcServer/handlers/blockheaders-stream/getHistoricalBlockHeadersIteratorFactory');
const subscribeToNewBlockHeaders = require('../lib/grpcServer/handlers/blockheaders-stream/subscribeToNewBlockHeaders');

const subscribeToNewTransactions = require('../lib/transactionsFilter/subscribeToNewTransactions');
const getHistoricalTransactionsIteratorFactory = require('../lib/transactionsFilter/getHistoricalTransactionsIteratorFactory');
const getMemPoolTransactionsFactory = require('../lib/transactionsFilter/getMemPoolTransactionsFactory');
const MasternodeListSync = require('../lib/MasternodeListSync');
const subscribeToMasternodeListHandlerFactory = require('../lib/grpcServer/handlers/core/subscribeToMasternodeListHandlerFactory');

async function main() {
  // Validate config
  const configValidationResult = validateConfig(config);
  if (!configValidationResult.isValid) {
    configValidationResult.validationErrors.forEach(logger.fatal.bind(logger));
    logger.error('Aborting DAPI startup due to config validation errors');
    process.exit();
  }

  const isProductionEnvironment = process.env.NODE_ENV === 'production';

  // Subscribe to events from Dash Core
  const dashCoreZmqClient = new ZmqClient(config.dashcore.zmq.host, config.dashcore.zmq.port);

  // Bind logs on ZMQ connection events
  dashCoreZmqClient.on(ZmqClient.events.DISCONNECTED, logger.warn.bind(logger));
  dashCoreZmqClient.on(ZmqClient.events.CONNECTION_DELAY, logger.warn.bind(logger));
  dashCoreZmqClient.on(ZmqClient.events.MONITOR_ERROR, logger.warn.bind(logger));

  // Wait until zmq connection is established
  logger.info(`Connecting to dashcore ZMQ on ${dashCoreZmqClient.connectionString}`);

  await dashCoreZmqClient.start();

  logger.info('Connection to ZMQ established.');

  // Add ZMQ event listeners
  const bloomFilterEmitterCollection = new BloomFilterEmitterCollection();
  const emitBlockEventToFilterCollection = emitBlockEventToFilterCollectionFactory(
    bloomFilterEmitterCollection,
  );
  const testRawTransactionAgainstFilterCollection = testTransactionAgainstFilterCollectionFactory(
    bloomFilterEmitterCollection,
  );
  const emitInstantLockToFilterCollection = emitInstantLockToFilterCollectionFactory(
    bloomFilterEmitterCollection,
  );

  // Send raw transactions via `subscribeToTransactionsWithProofs` stream if matched
  dashCoreZmqClient.on(
    dashCoreZmqClient.topics.rawtx,
    testRawTransactionAgainstFilterCollection,
  );

  // Send merkle blocks via `subscribeToTransactionsWithProofs` stream
  dashCoreZmqClient.on(
    dashCoreZmqClient.topics.rawblock,
    emitBlockEventToFilterCollection,
  );

  // TODO: check if we can receive this event before 'rawtx', and if we can,
  // we need to test tx in this message first before emitting lock to the bloom
  // filter collection
  // Send transaction instant locks via `subscribeToTransactionsWithProofs` stream
  dashCoreZmqClient.on(
    dashCoreZmqClient.topics.rawtxlocksig,
    emitInstantLockToFilterCollection,
  );

  const blockHeadersCache = new BlockHeadersCache();

  const chainDataProvider = new ChainDataProvider(
    dashCoreRpcClient,
    dashCoreZmqClient,
    blockHeadersCache,
  );

  await chainDataProvider.init();

  const masternodeListSync = new MasternodeListSync(
    dashCoreRpcClient,
    chainDataProvider,
    config.network,
  );

  await masternodeListSync.init();

  // Start GRPC server
  logger.info('Starting GRPC server');

  const wrapInErrorHandler = wrapInErrorHandlerFactory(logger, isProductionEnvironment);

  const getHistoricalTransactionsIterator = getHistoricalTransactionsIteratorFactory(
    dashCoreRpcClient,
  );

  const getHistoricalBlockHeadersIterator = getHistoricalBlockHeadersIteratorFactory(
    chainDataProvider,
  );

  const getMemPoolTransactions = getMemPoolTransactionsFactory(
    dashCoreRpcClient,
    testTransactionsAgainstFilter,
  );

  const subscribeToTransactionsWithProofsHandler = subscribeToTransactionsWithProofsHandlerFactory(
    getHistoricalTransactionsIterator,
    subscribeToNewTransactions,
    bloomFilterEmitterCollection,
    testTransactionsAgainstFilter,
    dashCoreRpcClient,
    getMemPoolTransactions,
    chainDataProvider,
  );

  const wrappedSubscribeToTransactionsWithProofs = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      TransactionsWithProofsRequest,
      PBJSTransactionsWithProofsRequest,
    ),
    protobufToJsonFactory(
      PBJSTransactionsWithProofsResponse,
    ),
    wrapInErrorHandler(subscribeToTransactionsWithProofsHandler),
  );

  // eslint-disable-next-line operator-linebreak
  const subscribeToBlockHeadersWithChainLocksHandler =
    subscribeToBlockHeadersWithChainLocksHandlerFactory(
      getHistoricalBlockHeadersIterator,
      dashCoreRpcClient,
      chainDataProvider,
      dashCoreZmqClient,
      subscribeToNewBlockHeaders,
    );

  const wrappedSubscribeToBlockHeadersWithChainLocks = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      BlockHeadersWithChainLocksRequest,
      PBJSBlockHeadersWithChainLocksRequest,
    ),
    protobufToJsonFactory(
      PBJSBlockHeadersWithChainLocksResponse,
    ),
    wrapInErrorHandler(subscribeToBlockHeadersWithChainLocksHandler),
  );

  const subscribeToMasternodeListHandler = subscribeToMasternodeListHandlerFactory(
    masternodeListSync,
  );

  const wrappedSubscribeToMasternodeListHandler = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      MasternodeListRequest,
      PBJSMasternodeListRequest,
    ),
    protobufToJsonFactory(
      PBJSMasternodeListResponse,
    ),
    wrapInErrorHandler(subscribeToMasternodeListHandler),
  );

  const grpcServer = createServer(
    getCoreDefinition(0),
    {
      subscribeToTransactionsWithProofs: wrappedSubscribeToTransactionsWithProofs,
      subscribeToBlockHeadersWithChainLocks: wrappedSubscribeToBlockHeadersWithChainLocks,
      subscribeToMasternodeList: wrappedSubscribeToMasternodeListHandler,
    },
  );

  grpcServer.bindAsync(
    `0.0.0.0:${config.txFilterStream.grpcServer.port}`,
    grpc.ServerCredentials.createInsecure(),
    () => {
      grpcServer.start();
    },
  );

  logger.info(`GRPC server is listening on port ${config.txFilterStream.grpcServer.port}`);

  // Display message that everything is ok
  logger.info(`DAPI TxFilterStream process is up and running in ${config.livenet ? 'livenet' : 'testnet'} mode`);
  logger.info(`Network is ${config.network}`);
}

main().catch((e) => {
  logger.error(e.stack);
  process.exit();
});

process.on('SIGINT', () => {
  logger.info('Received SIGINT. Exiting...');

  process.exit();
});

const Long = require('long');

const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asClass,
  asFunction,
  asValue,
} = require('awilix');

// eslint-disable-next-line import/no-unresolved
const level = require('level-rocksdb');

const LRUCache = require('lru-cache');

const RpcClient = require('@dashevo/dashd-rpc/promise');
const ZMQClient = require('@dashevo/dashd-zmq');

const DashPlatformProtocol = require('@dashevo/dpp');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const findMyWay = require('find-my-way');

const pino = require('pino');

const sanitizeUrl = require('./util/sanitizeUrl');

const LatestCoreChainLock = require('./core/LatestCoreChainLock');
const IdentityLevelDBRepository = require('./identity/IdentityLevelDBRepository');
const PublicKeyIdentityIdMapLevelDBRepository = require(
  './identity/PublicKeyIdentityIdMapLevelDBRepository',
);

const DataContractLevelDBRepository = require('./dataContract/DataContractLevelDBRepository');

const findNotIndexedOrderByFields = require('./document/query/findNotIndexedOrderByFields');
const findNotIndexedFields = require('./document/query/findNotIndexedFields');
const getIndexedFieldsFromDocumentSchema = require('./document/query/getIndexedFieldsFromDocumentSchema');
const findConflictingConditions = require('./document/query/findConflictingConditions');

const getDocumentDatabaseFactory = require('./document/mongoDbRepository/getDocumentDatabaseFactory');
const validateQueryFactory = require('./document/query/validateQueryFactory');
const convertWhereToMongoDbQuery = require('./document/mongoDbRepository/convertWhereToMongoDbQuery');
const createDocumentMongoDbRepositoryFactory = require('./document/mongoDbRepository/createDocumentMongoDbRepositoryFactory');
const DocumentDatabaseManager = require('./document/mongoDbRepository/DocumentDatabaseManager');
const convertToMongoDbIndicesFunction = require('./document/mongoDbRepository/convertToMongoDbIndices');
const fetchDocumentsFactory = require('./document/fetchDocumentsFactory');
const MongoDBTransaction = require('./mongoDb/MongoDBTransaction');
const connectToMongoDBFactory = require('./mongoDb/connectToMongoDBFactory');
const waitReplicaSetInitializeFactory = require('./mongoDb/waitReplicaSetInitializeFactory');

const BlockExecutionState = require('./blockchainState/blockExecution/BlockExecutionState');
const BlockchainStateLevelDBRepository = require('./blockchainState/BlockchainStateLevelDBRepository');
const BlockExecutionDBTransactions = require('./blockchainState/blockExecution/BlockExecutionDBTransactions');

const createIsolatedValidatorSnapshot = require('./dpp/isolation/createIsolatedValidatorSnapshot');
const createIsolatedDppFactory = require('./dpp/isolation/createIsolatedDppFactory');
const unserializeStateTransitionFactory = require(
  './abci/handlers/stateTransition/unserializeStateTransitionFactory',
);

const DriveStateRepository = require('./dpp/DriveStateRepository');
const CachedStateRepositoryDecorator = require('./dpp/CachedStateRepositoryDecorator');
const LoggedStateRepositoryDecorator = require('./dpp/LoggedStateRepositoryDecorator');

const dataContractQueryHandlerFactory = require('./abci/handlers/query/dataContractQueryHandlerFactory');
const identityQueryHandlerFactory = require('./abci/handlers/query/identityQueryHandlerFactory');
const documentQueryHandlerFactory = require('./abci/handlers/query/documentQueryHandlerFactory');
const identitiesByPublicKeyHashesQueryHandlerFactory = require('./abci/handlers/query/identitiesByPublicKeyHashesQueryHandlerFactory');
const identityIdsByPublicKeyHashesQueryHandlerFactory = require('./abci/handlers/query/identityIdsByPublicKeyHashesQueryHandlerFactory');

const wrapInErrorHandlerFactory = require('./abci/errors/wrapInErrorHandlerFactory');
const errorHandler = require('./errorHandler');

const checkTxHandlerFactory = require('./abci/handlers/checkTxHandlerFactory');
const commitHandlerFactory = require('./abci/handlers/commitHandlerFactory');
const deliverTxHandlerFactory = require('./abci/handlers/deliverTxHandlerFactory');
const infoHandlerFactory = require('./abci/handlers/infoHandlerFactory');
const beginBlockHandlerFactory = require('./abci/handlers/beginBlockHandlerFactory');
const endBlockHandlerFactory = require('./abci/handlers/endBlockHandlerFactory');
const queryHandlerFactory = require('./abci/handlers/queryHandlerFactory');

const waitForCoreSyncFactory = require('./core/waitForCoreSyncFactory');
const waitForCoreChainLockSyncFallback = require('./core/waitForCoreChainLockSyncFallbackFactory');
const waitForCoreChainLockSyncFactory = require('./core/waitForCoreChainLockSyncFactory');
const detectStandaloneRegtestModeFactory = require('./core/detectStandaloneRegtestModeFactory');

/**
 *
 * @param {Object} options
 * @param {string} options.ABCI_HOST
 * @param {string} options.ABCI_PORT
 * @param {string} options.BLOCKCHAIN_STATE_LEVEL_DB_FILE
 * @param {string} options.DATA_CONTRACT_CACHE_SIZE
 * @param {string} options.IDENTITY_LEVEL_DB_FILE
 * @param {string} options.ISOLATED_ST_UNSERIALIZATION_MEMORY_LIMIT
 * @param {string} options.ISOLATED_ST_UNSERIALIZATION_TIMEOUT_MILLIS
 * @param {string} options.DATA_CONTRACT_LEVEL_DB_FILE
 * @param {string} options.DOCUMENT_MONGODB_DB_PREFIX
 * @param {string} options.DOCUMENT_MONGODB_URL
 * @param {string} options.CORE_JSON_RPC_HOST
 * @param {string} options.CORE_JSON_RPC_PORT
 * @param {string} options.CORE_JSON_RPC_USERNAME
 * @param {string} options.CORE_JSON_RPC_PASSWORD
 * @param {string} options.CORE_ZMQ_HOST
 * @param {string} options.CORE_ZMQ_PORT
 * @param {string} options.IDENTITY_SKIP_ASSET_LOCK_CONFIRMATION_VALIDATION
 * @param {string} options.DPNS_CONTRACT_BLOCK_HEIGHT
 * @param {string} options.DPNS_CONTRACT_ID
 * @param {string} options.LOGGING_LEVEL
 * @param {string} options.NODE_ENV
 *
 * @return {AwilixContainer}
 */
async function createDIContainer(options) {
  if (options.DPNS_CONTRACT_ID && !options.DPNS_CONTRACT_BLOCK_HEIGHT) {
    throw new Error('DPNS_CONTRACT_BLOCK_HEIGHT must be set');
  }

  const container = createAwilixContainer({
    injectionMode: InjectionMode.CLASSIC,
  });

  /**
   * Register protocolVersion
   * Define highest supported protocol version
   */
  container.register({
    protocolVersion: asValue(Long.fromInt(0)),
  });

  /**
   * Register environment variables
   */
  container.register({
    abciHost: asValue(options.ABCI_HOST),
    abciPort: asValue(options.ABCI_PORT),
    blockchainStateLevelDBFile: asValue(options.BLOCKCHAIN_STATE_LEVEL_DB_FILE),
    dataContractCacheSize: asValue(options.DATA_CONTRACT_CACHE_SIZE),
    identityLevelDBFile: asValue(options.IDENTITY_LEVEL_DB_FILE),
    isolatedSTUnserializationMemoryLimit: asValue(
      parseInt(options.ISOLATED_ST_UNSERIALIZATION_MEMORY_LIMIT, 10),
    ),
    isolatedSTUnserializationTimeout: asValue(
      parseInt(options.ISOLATED_ST_UNSERIALIZATION_TIMEOUT_MILLIS, 10),
    ),
    dataContractLevelDBFile: asValue(options.DATA_CONTRACT_LEVEL_DB_FILE),
    documentMongoDBPrefix: asValue(options.DOCUMENT_MONGODB_DB_PREFIX),
    documentMongoDBUrl: asValue(options.DOCUMENT_MONGODB_URL),
    chainLock: asValue(undefined),
    coreJsonRpcHost: asValue(options.CORE_JSON_RPC_HOST),
    coreJsonRpcPort: asValue(options.CORE_JSON_RPC_PORT),
    coreJsonRpcUsername: asValue(options.CORE_JSON_RPC_USERNAME),
    coreJsonRpcPassword: asValue(options.CORE_JSON_RPC_PASSWORD),
    coreZMQHost: asValue(options.CORE_ZMQ_HOST),
    coreZMQPort: asValue(options.CORE_ZMQ_PORT),
    dpnsContractBlockHeight: asValue(parseInt(options.DPNS_CONTRACT_BLOCK_HEIGHT, 10)),
    dpnsContractId: asValue(
      options.DPNS_CONTRACT_ID
        ? Identifier.from(options.DPNS_CONTRACT_ID)
        : undefined,
    ),
    loggingLevel: asValue(options.LOGGING_LEVEL),
    isProductionEnvironment: asValue(options.NODE_ENV === 'production'),
    maxIdentitiesPerRequest: asValue(25),
  });

  /**
   * Register global DPP options
   */
  container.register({
    dppOptions: asValue({
      identities: {
        skipAssetLockConfirmationValidation: options
          .IDENTITY_SKIP_ASSET_LOCK_CONFIRMATION_VALIDATION === 'true',
      },
    }),
  });

  /**
   * Register Core related
   */
  container.register({
    latestCoreChainLock: asClass(LatestCoreChainLock).singleton(),
  });

  /**
   * Register common services
   */
  container.register({
    logger: asFunction((loggingLevel) => pino({
      level: loggingLevel,
      prettyPrint: {
        translateTime: true,
      },
    })),

    noStateDpp: asFunction((dppOptions) => (
      new DashPlatformProtocol({
        ...dppOptions,
        stateRepository: undefined,
      })
    )).singleton(),

    sanitizeUrl: asValue(sanitizeUrl),
    coreZMQClient: asFunction((
      coreZMQHost,
      coreZMQPort,
    ) => (
      new ZMQClient({
        protocol: 'tcp',
        host: coreZMQHost,
        port: coreZMQPort,
      }))).singleton(),

    coreRpcClient: asFunction((
      coreJsonRpcHost,
      coreJsonRpcPort,
      coreJsonRpcUsername,
      coreJsonRpcPassword,
    ) => (
      new RpcClient({
        protocol: 'http',
        host: coreJsonRpcHost,
        port: coreJsonRpcPort,
        user: coreJsonRpcUsername,
        pass: coreJsonRpcPassword,
      }))).singleton(),
  });

  /**
   * Register Identity
   */
  container.register({
    identityLevelDB: asFunction((identityLevelDBFile) => (
      level(identityLevelDBFile, { keyEncoding: 'binary', valueEncoding: 'binary' })
    )).disposer((levelDB) => levelDB.close())
      .singleton(),

    identityRepository: asClass(IdentityLevelDBRepository).singleton(),
    identityTransaction: asFunction((identityRepository) => (
      identityRepository.createTransaction()
    )).singleton(),

    publicKeyIdentityIdRepository: asClass(PublicKeyIdentityIdMapLevelDBRepository).singleton(),
  });

  /**
   * Register Data Contract
   */
  container.register({
    dataContractLevelDB: asFunction((dataContractLevelDBFile) => (
      level(dataContractLevelDBFile, { keyEncoding: 'binary', valueEncoding: 'binary' })
    )).disposer((levelDB) => levelDB.close())
      .singleton(),

    dataContractRepository: asClass(DataContractLevelDBRepository).singleton(),
    dataContractTransaction: asFunction((dataContractRepository) => (
      dataContractRepository.createTransaction()
    )).singleton(),

    dataContractCache: asFunction((dataContractCacheSize) => (
      new LRUCache(dataContractCacheSize)
    )).singleton(),
  });

  /**
   * Register Document
   */
  container.register({
    connectToDocumentMongoDB: asFunction((documentMongoDBUrl) => (
      connectToMongoDBFactory(documentMongoDBUrl)
    )).singleton(),

    findConflictingConditions: asValue(findConflictingConditions),
    getIndexedFieldsFromDocumentSchema: asValue(getIndexedFieldsFromDocumentSchema),
    findNotIndexedFields: asValue(findNotIndexedFields),
    findNotIndexedOrderByFields: asValue(findNotIndexedOrderByFields),
    convertWhereToMongoDbQuery: asValue(convertWhereToMongoDbQuery),
    validateQuery: asFunction(validateQueryFactory).singleton(),

    convertToMongoDbIndices: asValue(convertToMongoDbIndicesFunction),

    getDocumentDatabase: asFunction(getDocumentDatabaseFactory).singleton(),
    createDocumentRepository: asFunction(createDocumentMongoDbRepositoryFactory).singleton(),
    documentDatabaseManager: asFunction((
      createDocumentRepository,
      convertToMongoDbIndices,
      getDocumentDatabase,
    ) => new DocumentDatabaseManager(
      createDocumentRepository,
      convertToMongoDbIndices,
      getDocumentDatabase,
    )),
    documentTransaction: asClass(MongoDBTransaction).singleton(),
    fetchDocuments: asFunction(fetchDocumentsFactory).singleton(),
  });

  /**
   * Register blockchain state
   */
  container.register({
    blockchainStateLevelDB: asFunction((blockchainStateLevelDBFile) => (
      level(blockchainStateLevelDBFile, { keyEncoding: 'binary', valueEncoding: 'binary' })
    )).disposer((levelDB) => levelDB.close())
      .singleton(),

    blockchainStateRepository: asClass(BlockchainStateLevelDBRepository).singleton(),
  });

  const blockchainStateRepository = container.resolve('blockchainStateRepository');
  const blockchainState = await blockchainStateRepository.fetch();

  container.register({
    blockchainState: asValue(blockchainState),
    blockExecutionDBTransactions: asClass(BlockExecutionDBTransactions).singleton(),
    blockExecutionState: asClass(BlockExecutionState).singleton(),
  });

  /**
   * Register DPP
   */
  const isolatedSnapshot = await createIsolatedValidatorSnapshot();

  container.register({
    isolatedSTUnserializationOptions: asFunction((
      isolatedSTUnserializationMemoryLimit,
      isolatedSTUnserializationTimeout,
    ) => ({
      memoryLimit: isolatedSTUnserializationMemoryLimit,
      timeout: isolatedSTUnserializationTimeout,
    })).singleton(),

    isolatedJsonSchemaValidatorSnapshot: asValue(isolatedSnapshot),

    stateRepository: asFunction((
      identityRepository,
      publicKeyIdentityIdRepository,
      dataContractRepository,
      fetchDocuments,
      createDocumentRepository,
      coreRpcClient,
      dataContractCache,
      blockExecutionState,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        createDocumentRepository,
        coreRpcClient,
        blockExecutionState,
      );

      return new CachedStateRepositoryDecorator(
        stateRepository,
        dataContractCache,
      );
    }).singleton(),

    transactionalStateRepository: asFunction((
      identityRepository,
      publicKeyIdentityIdRepository,
      dataContractRepository,
      fetchDocuments,
      createDocumentRepository,
      coreRpcClient,
      blockExecutionDBTransactions,
      dataContractCache,
      blockExecutionState,
      logger,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        createDocumentRepository,
        coreRpcClient,
        blockExecutionState,
        blockExecutionDBTransactions,
      );

      const cachedRepository = new CachedStateRepositoryDecorator(
        stateRepository, dataContractCache,
      );

      return new LoggedStateRepositoryDecorator(
        cachedRepository,
        logger,
      );
    }).singleton(),

    unserializeStateTransition: asFunction((
      isolatedJsonSchemaValidatorSnapshot,
      isolatedSTUnserializationOptions,
      stateRepository,
      dppOptions,
    ) => {
      const createIsolatedDpp = createIsolatedDppFactory(
        isolatedJsonSchemaValidatorSnapshot,
        isolatedSTUnserializationOptions,
        stateRepository,
        dppOptions,
      );

      return unserializeStateTransitionFactory(createIsolatedDpp);
    }).singleton(),

    transactionalUnserializeStateTransition: asFunction((
      isolatedJsonSchemaValidatorSnapshot,
      isolatedSTUnserializationOptions,
      transactionalStateRepository,
      dppOptions,
    ) => {
      const createIsolatedDpp = createIsolatedDppFactory(
        isolatedJsonSchemaValidatorSnapshot,
        isolatedSTUnserializationOptions,
        transactionalStateRepository,
        dppOptions,
      );

      return unserializeStateTransitionFactory(createIsolatedDpp);
    }).singleton(),

    transactionalDpp: asFunction((transactionalStateRepository, dppOptions) => (
      new DashPlatformProtocol({
        ...dppOptions,
        stateRepository: transactionalStateRepository,
      })
    )).singleton(),
  });

  /**
   * Register check functions
   */
  container.register({
    detectStandaloneRegtestMode: asFunction(detectStandaloneRegtestModeFactory).singleton(),
  });
  container.register({
    waitForCoreSync: asFunction(waitForCoreSyncFactory).singleton(),
  });
  container.register({
    waitForCoreChainLockSyncFallback: asFunction(waitForCoreChainLockSyncFallback).singleton(),
  });
  container.register({
    waitForCoreChainLockSync: asFunction(waitForCoreChainLockSyncFactory).singleton(),
  });
  container.register({
    waitReplicaSetInitialize: asFunction(waitReplicaSetInitializeFactory).singleton(),
  });

  /**
   * Register ABCI handlers
   */
  container.register({
    identityQueryHandler: asFunction(identityQueryHandlerFactory).singleton(),
    dataContractQueryHandler: asFunction(dataContractQueryHandlerFactory).singleton(),
    documentQueryHandler: asFunction(documentQueryHandlerFactory).singleton(),
    identitiesByPublicKeyHashesQueryHandler:
      asFunction(identitiesByPublicKeyHashesQueryHandlerFactory).singleton(),
    identityIdsByPublicKeyHashesQueryHandler:
      asFunction(identityIdsByPublicKeyHashesQueryHandlerFactory).singleton(),

    queryHandlerRouter: asFunction((
      identityQueryHandler,
      dataContractQueryHandler,
      documentQueryHandler,
      identitiesByPublicKeyHashesQueryHandler,
      identityIdsByPublicKeyHashesQueryHandler,
    ) => {
      const router = findMyWay({
        ignoreTrailingSlash: true,
      });

      router.on('GET', '/identities', identityQueryHandler);
      router.on('GET', '/dataContracts', dataContractQueryHandler);
      router.on('GET', '/dataContracts/documents', documentQueryHandler);
      router.on('GET', '/identities/by-public-key-hash', identitiesByPublicKeyHashesQueryHandler);
      router.on('GET', '/identities/by-public-key-hash/id', identityIdsByPublicKeyHashesQueryHandler);

      return router;
    }).singleton(),

    infoHandler: asFunction(infoHandlerFactory).singleton(),
    checkTxHandler: asFunction(checkTxHandlerFactory).singleton(),
    beginBlockHandler: asFunction(beginBlockHandlerFactory).singleton(),
    deliverTxHandler: asFunction(deliverTxHandlerFactory).singleton(),
    endBlockHandler: asFunction(endBlockHandlerFactory).singleton(),
    commitHandler: asFunction(commitHandlerFactory).singleton(),
    queryHandler: asFunction(queryHandlerFactory).singleton(),

    wrapInErrorHandler: asFunction(wrapInErrorHandlerFactory).singleton(),
    errorHandler: asValue(errorHandler),

    abciHandlers: asFunction((
      infoHandler,
      checkTxHandler,
      beginBlockHandler,
      deliverTxHandler,
      endBlockHandler,
      commitHandler,
      wrapInErrorHandler,
      queryHandler,
    ) => ({
      info: infoHandler,
      checkTx: wrapInErrorHandler(checkTxHandler, { respondWithInternalError: true }),
      beginBlock: beginBlockHandler,
      deliverTx: wrapInErrorHandler(deliverTxHandler),
      endBlock: wrapInErrorHandler(endBlockHandler, { respondWithInternalError: true }),
      commit: commitHandler,
      query: wrapInErrorHandler(queryHandler, { respondWithInternalError: true }),
    })).singleton(),
  });

  return container;
}

module.exports = createDIContainer;

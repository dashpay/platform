const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asClass,
  asFunction,
  asValue,
} = require('awilix');

const Long = require('long');

const Merk = require('@dashevo/merk');

const LRUCache = require('lru-cache');
const RpcClient = require('@dashevo/dashd-rpc/promise');

const ZMQClient = require('@dashevo/dashd-zmq');
const DashPlatformProtocol = require('@dashevo/dpp');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const findMyWay = require('find-my-way');

const pino = require('pino');

const sanitizeUrl = require('./util/sanitizeUrl');
const LatestCoreChainLock = require('./core/LatestCoreChainLock');

const MerkDbStore = require('./merkDb/MerkDbStore');
const RootTree = require('./rootTree/RootTree');
const CommonStoreRootTreeLeaf = require('./rootTree/CommonStoreRootTreeLeaf');

const IdentityStoreRepository = require('./identity/IdentityStoreRepository');
const IdentitiesStoreRootTreeLeaf = require('./identity/IdentitiesStoreRootTreeLeaf');
const PublicKeyToIdentityIdStoreRepository = require(
  './identity/PublicKeyToIdentityIdStoreRepository',
);
const PublicKeyToIdentityIdStoreRootTreeLeaf = require(
  './identity/PublicKeyToIdentityIdStoreRootTreeLeaf',
);

const DataContractStoreRepository = require('./dataContract/DataContractStoreRepository');
const DataContractsStoreRootTreeLeaf = require('./dataContract/DataContractsStoreRootTreeLeaf');

const findNotIndexedOrderByFields = require('./document/query/findNotIndexedOrderByFields');
const findNotIndexedFields = require('./document/query/findNotIndexedFields');
const getIndexedFieldsFromDocumentSchema = require('./document/query/getIndexedFieldsFromDocumentSchema');

const DocumentsStoreRootTreeLeaf = require('./document/DocumentsStoreRootTreeLeaf');
const DocumentDbTransaction = require('./document/DocumentsDbTransaction');
const DocumentStoreRepository = require('./document/DocumentStoreRepository');
const DocumentIndexedStoreRepository = require('./document/DocumentIndexedStoreRepository');
const findConflictingConditions = require('./document/query/findConflictingConditions');
const getDocumentDatabaseFactory = require('./document/mongoDbRepository/getDocumentMongoDbDatabaseFactory');
const validateQueryFactory = require('./document/query/validateQueryFactory');
const convertWhereToMongoDbQuery = require('./document/mongoDbRepository/convertWhereToMongoDbQuery');
const createDocumentMongoDbRepositoryFactory = require('./document/mongoDbRepository/createDocumentMongoDbRepositoryFactory');
const DocumentDatabaseManager = require('./document/mongoDbRepository/DocumentDatabaseManager');
const convertToMongoDbIndicesFunction = require('./document/mongoDbRepository/convertToMongoDbIndices');
const fetchDocumentsFactory = require('./document/fetchDocumentsFactory');
const MongoDBTransaction = require('./mongoDb/MongoDBTransaction');
const connectToMongoDBFactory = require('./mongoDb/connectToMongoDBFactory');

const waitReplicaSetInitializeFactory = require('./mongoDb/waitReplicaSetInitializeFactory');
const BlockExecutionContext = require('./blockExecution/BlockExecutionContext');
const ChainInfoCommonStoreRepository = require('./chainInfo/ChainInfoCommonStoreRepository');

const BlockExecutionDBTransactions = require('./blockExecution/BlockExecutionDBTransactions');
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
const verifyChainLockQueryHandlerFactory = require('./abci/handlers/query/verifyChainLockQueryHandlerFactory');

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
const waitForCoreChainLockSyncFallbackFactory = require('./core/waitForCoreChainLockSyncFallbackFactory');
const waitForCoreChainLockSyncFactory = require('./core/waitForCoreChainLockSyncFactory');
const detectStandaloneRegtestModeFactory = require('./core/detectStandaloneRegtestModeFactory');
const waitForSMLSyncFactory = require('./core/waitForSMLSyncFactory');
const SimplifiedMasternodeList = require('./core/SimplifiedMasternodeList');
const decodeChainLock = require('./core/decodeChainLock');

/**
 *
 * @param {Object} options
 * @param {string} options.ABCI_HOST
 * @param {string} options.ABCI_PORT
 * @param {string} options.COMMON_STORE_MERK_DB_FILE
 * @param {string} options.DATA_CONTRACT_CACHE_SIZE
 * @param {string} options.IDENTITIES_STORE_MERK_DB_FILE
 * @param {string} options.PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE
 * @param {string} options.ISOLATED_ST_UNSERIALIZATION_MEMORY_LIMIT
 * @param {string} options.ISOLATED_ST_UNSERIALIZATION_TIMEOUT_MILLIS
 * @param {string} options.DATA_CONTRACTS_STORE_MERK_DB_FILE
 * @param {string} options.DOCUMENTS_STORE_MERK_DB_FILE
 * @param {string} options.DOCUMENT_MONGODB_DB_PREFIX
 * @param {string} options.DOCUMENT_MONGODB_URL
 * @param {string} options.CORE_JSON_RPC_HOST
 * @param {string} options.CORE_JSON_RPC_PORT
 * @param {string} options.CORE_JSON_RPC_USERNAME
 * @param {string} options.CORE_JSON_RPC_PASSWORD
 * @param {string} options.CORE_ZMQ_HOST
 * @param {string} options.CORE_ZMQ_PORT
 * @param {string} options.NETWORK
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
   * Register itself (usually to solve recursive dependencies)
   */
  container.register({
    container: asValue(container),
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
    commonStoreMerkDBFile: asValue(options.COMMON_STORE_MERK_DB_FILE),
    dataContractCacheSize: asValue(options.DATA_CONTRACT_CACHE_SIZE),
    publicKeyToIdentityIdStoreMerkDBFile:
      asValue(options.PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE),
    identitiesStoreMerkDBFile: asValue(options.IDENTITIES_STORE_MERK_DB_FILE),
    isolatedSTUnserializationMemoryLimit: asValue(
      parseInt(options.ISOLATED_ST_UNSERIALIZATION_MEMORY_LIMIT, 10),
    ),
    isolatedSTUnserializationTimeout: asValue(
      parseInt(options.ISOLATED_ST_UNSERIALIZATION_TIMEOUT_MILLIS, 10),
    ),
    dataContractsStoreMerkDBFile: asValue(options.DATA_CONTRACTS_STORE_MERK_DB_FILE),
    documentsStoreMerkDBFile: asValue(options.DOCUMENTS_STORE_MERK_DB_FILE),
    documentMongoDBPrefix: asValue(options.DOCUMENT_MONGODB_DB_PREFIX),
    documentMongoDBUrl: asValue(options.DOCUMENT_MONGODB_URL),
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
    network: asValue(options.NETWORK),
    loggingLevel: asValue(options.LOGGING_LEVEL),
    isProductionEnvironment: asValue(options.NODE_ENV === 'production'),
    maxIdentitiesPerRequest: asValue(25),
    smlMaxListsLimit: asValue(16),
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
    latestCoreChainLock: asValue(new LatestCoreChainLock()),
    simplifiedMasternodeList: asClass(SimplifiedMasternodeList).proxy().singleton(),
    decodeChainLock: asValue(decodeChainLock),
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
    publicKeyToIdentityIdStore: asFunction((publicKeyToIdentityIdStoreMerkDBFile) => {
      const merkDb = new Merk(publicKeyToIdentityIdStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();
      merkDb.db.close();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      }
    }).singleton(),

    publicKeyToIdentityIdStoreRootTreeLeaf: asClass(PublicKeyToIdentityIdStoreRootTreeLeaf),

    identitiesStore: asFunction((identitiesStoreMerkDBFile) => {
      const merkDb = new Merk(identitiesStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();
      merkDb.db.close();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      }
    }).singleton(),

    identitiesStoreRootTreeLeaf: asClass(IdentitiesStoreRootTreeLeaf).singleton(),

    identityRepository: asClass(IdentityStoreRepository).singleton(),
    identitiesTransaction: asFunction((identitiesStore) => (
      identitiesStore.createTransaction()
    )).singleton(),

    publicKeyToIdentityIdRepository: asClass(PublicKeyToIdentityIdStoreRepository).singleton(),
  });

  /**
   * Register Data Contract
   */
  container.register({
    dataContractsStore: asFunction((dataContractsStoreMerkDBFile) => {
      const merkDb = new Merk(dataContractsStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();
      merkDb.db.close();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      }
    }).singleton(),

    dataContractsStoreRootTreeLeaf: asClass(DataContractsStoreRootTreeLeaf).singleton(),

    dataContractRepository: asClass(DataContractStoreRepository).singleton(),
    dataContractsTransaction: asFunction((dataContractsStore) => (
      dataContractsStore.createTransaction()
    )).singleton(),

    dataContractCache: asFunction((dataContractCacheSize) => (
      new LRUCache(dataContractCacheSize)
    )).singleton(),
  });

  /**
   * Register Document
   */
  container.register({
    documentsStore: asFunction((documentsStoreMerkDBFile) => {
      const merkDb = new Merk(documentsStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();
      merkDb.db.close();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      }
    }).singleton(),

    documentStoreRepository: asClass(DocumentStoreRepository).singleton(),

    documentsStoreRootTreeLeaf: asClass(DocumentsStoreRootTreeLeaf).singleton(),

    documentRepository: asClass(DocumentIndexedStoreRepository).singleton(),

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

    getDocumentMongoDBDatabase: asFunction(getDocumentDatabaseFactory).singleton(),
    createDocumentMongoDbRepository: asFunction(createDocumentMongoDbRepositoryFactory).singleton(),
    documentDatabaseManager: asClass(DocumentDatabaseManager),
    documentMongoDBTransaction: asClass(MongoDBTransaction).singleton(),
    documentsStoreTransaction: asFunction((documentsStore) => (
      documentsStore.createTransaction()
    )).singleton(),
    documentsDbTransaction: asClass(DocumentDbTransaction).singleton(),
    fetchDocuments: asFunction(fetchDocumentsFactory).singleton(),
  });

  /**
   * Register chain info
   */
  container.register({
    commonStore: asFunction((commonStoreMerkDBFile) => {
      const merkDb = new Merk(commonStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();
      merkDb.db.close();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      }
    }).singleton(),

    commonStoreRootTreeLeaf: asClass(CommonStoreRootTreeLeaf).singleton(),

    chainInfoRepository: asClass(ChainInfoCommonStoreRepository).singleton(),
  });

  const chainInfoRepository = container.resolve('chainInfoRepository');
  const chainInfo = await chainInfoRepository.fetch();

  container.register({
    chainInfo: asValue(chainInfo),
  });

  /**
   * Register block execution context
   */
  container.register({
    blockExecutionDBTransactions: asClass(BlockExecutionDBTransactions).singleton(),
    blockExecutionContext: asClass(BlockExecutionContext).singleton(),
  });

  /**
   * Register root tree
   */
  container.register({
    rootTree: asFunction((
      commonStoreRootTreeLeaf,
      identitiesStoreRootTreeLeaf,
      publicKeyToIdentityIdStoreRootTreeLeaf,
      dataContractsStoreRootTreeLeaf,
      documentsStoreRootTreeLeaf,
    ) => (
      new RootTree([
        commonStoreRootTreeLeaf,
        identitiesStoreRootTreeLeaf,
        publicKeyToIdentityIdStoreRootTreeLeaf,
        dataContractsStoreRootTreeLeaf,
        documentsStoreRootTreeLeaf,
      ])
    )).singleton(),
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
      publicKeyToIdentityIdRepository,
      dataContractRepository,
      fetchDocuments,
      documentRepository,
      coreRpcClient,
      dataContractCache,
      blockExecutionContext,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyToIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        documentRepository,
        coreRpcClient,
        blockExecutionContext,
      );

      return new CachedStateRepositoryDecorator(
        stateRepository,
        dataContractCache,
      );
    }).singleton(),

    transactionalStateRepository: asFunction((
      identityRepository,
      publicKeyToIdentityIdRepository,
      dataContractRepository,
      fetchDocuments,
      documentRepository,
      coreRpcClient,
      blockExecutionDBTransactions,
      dataContractCache,
      blockExecutionContext,
      logger,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyToIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        documentRepository,
        coreRpcClient,
        blockExecutionContext,
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

    dpp: asFunction((stateRepository, dppOptions) => (
      new DashPlatformProtocol({
        ...dppOptions,
        stateRepository,
      })
    )).singleton(),

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
    waitForCoreChainLockSyncFallback:
      asFunction(waitForCoreChainLockSyncFallbackFactory).singleton(),
  });
  container.register({
    waitForSMLSync: asFunction(waitForSMLSyncFactory).singleton(),
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
    verifyChainLockQueryHandler: asFunction(verifyChainLockQueryHandlerFactory).singleton(),

    queryHandlerRouter: asFunction((
      identityQueryHandler,
      dataContractQueryHandler,
      documentQueryHandler,
      identitiesByPublicKeyHashesQueryHandler,
      identityIdsByPublicKeyHashesQueryHandler,
      verifyChainLockQueryHandler,
    ) => {
      const router = findMyWay({
        ignoreTrailingSlash: true,
      });

      router.on('GET', '/identities', identityQueryHandler);
      router.on('GET', '/dataContracts', dataContractQueryHandler);
      router.on('GET', '/dataContracts/documents', documentQueryHandler);
      router.on('GET', '/identities/by-public-key-hash', identitiesByPublicKeyHashesQueryHandler);
      router.on('GET', '/identities/by-public-key-hash/id', identityIdsByPublicKeyHashesQueryHandler);
      router.on('GET', '/verify-chainlock', verifyChainLockQueryHandler);

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
      endBlock: endBlockHandler,
      commit: commitHandler,
      query: wrapInErrorHandler(queryHandler, { respondWithInternalError: true }),
    })).singleton(),
  });

  return container;
}

module.exports = createDIContainer;

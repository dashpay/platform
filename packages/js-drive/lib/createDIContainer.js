const {
  createContainer: createAwilixContainer,
  InjectionMode,
  asClass,
  asFunction,
  asValue,
} = require('awilix');

const fs = require('fs');

const Long = require('long');

// eslint-disable-next-line import/no-unresolved
const level = require('level-rocksdb');

const { Merk } = require('@dashevo/merk');

const LRUCache = require('lru-cache');
const RpcClient = require('@dashevo/dashd-rpc/promise');

const { PublicKey } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const findMyWay = require('find-my-way');

const pino = require('pino');
const pinoMultistream = require('pino-multi-stream');

const createABCIServer = require('@dashevo/abci');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const featureFlagsSystemIds = require('@dashevo/feature-flags-contract/lib/systemIds');
const featureFlagsDocuments = require('@dashevo/feature-flags-contract/schema/feature-flags-documents.json');

const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const dpnsDocuments = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');

const masternodeRewardsSystemIds = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const masternodeRewardsDocuments = require('@dashevo/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json');

const dashpaySystemIds = require('@dashevo/dashpay-contract/lib/systemIds');
const dashpayDocuments = require('@dashevo/dashpay-contract/schema/dashpay.schema.json');

const packageJSON = require('../package.json');

const ZMQClient = require('./core/ZmqClient');

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

const DocumentsStoreRootTreeLeaf = require('./document/DocumentsStoreRootTreeLeaf');
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
const connectToMongoDBFactory = require('./mongoDb/connectToMongoDBFactory');

const waitReplicaSetInitializeFactory = require('./mongoDb/waitReplicaSetInitializeFactory');
const BlockExecutionContext = require('./blockExecution/BlockExecutionContext');
const CreditsDistributionPoolCommonStoreRepository = require('./creditsDistributionPool/CreditsDistributionPoolCommonStoreRepository');

const BlockExecutionStoreTransactions = require('./blockExecution/BlockExecutionStoreTransactions');
const PreviousBlockExecutionStoreTransactionsRepository = require('./blockExecution/PreviousBlockExecutionStoreTransactionsRepository');

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
const getProofsQueryHandlerFactory = require('./abci/handlers/query/getProofsQueryHandlerFactory');

const identityIdsByPublicKeyHashesQueryHandlerFactory = require('./abci/handlers/query/identityIdsByPublicKeyHashesQueryHandlerFactory');
const verifyChainLockQueryHandlerFactory = require('./abci/handlers/query/verifyChainLockQueryHandlerFactory');

const wrapInErrorHandlerFactory = require('./abci/errors/wrapInErrorHandlerFactory');

const errorHandlerFactory = require('./errorHandlerFactory');
const checkTxHandlerFactory = require('./abci/handlers/checkTxHandlerFactory');
const commitHandlerFactory = require('./abci/handlers/commitHandlerFactory');
const deliverTxHandlerFactory = require('./abci/handlers/deliverTxHandlerFactory');
const initChainHandlerFactory = require('./abci/handlers/initChainHandlerFactory');
const infoHandlerFactory = require('./abci/handlers/infoHandlerFactory');
const beginBlockHandlerFactory = require('./abci/handlers/beginBlockHandlerFactory');
const endBlockHandlerFactory = require('./abci/handlers/endBlockHandlerFactory');

const queryHandlerFactory = require('./abci/handlers/queryHandlerFactory');
const waitForCoreSyncFactory = require('./core/waitForCoreSyncFactory');
const waitForCoreChainLockSyncFactory = require('./core/waitForCoreChainLockSyncFactory');
const updateSimplifiedMasternodeListFactory = require('./core/updateSimplifiedMasternodeListFactory');
const waitForChainLockedHeightFactory = require('./core/waitForChainLockedHeightFactory');
const SimplifiedMasternodeList = require('./core/SimplifiedMasternodeList');
const decodeChainLock = require('./core/decodeChainLock');
const populateMongoDbTransactionFromObjectFactory = require('./document/populateMongoDbTransactionFromObjectFactory');

const FileDb = require('./fileDb/FileDb');
const SpentAssetLockTransactionsRepository = require('./identity/SpentAssetLockTransactionsRepository');
const SpentAssetLockTransactionsStoreRootTreeLeaf = require('./identity/SpentAssetLockTransactionsStoreRootTreeLeaf');
const cloneToPreviousStoreTransactionsFactory = require('./blockExecution/cloneToPreviousStoreTransactionsFactory');
const enrichErrorWithConsensusErrorFactory = require('./abci/errors/enrichErrorWithConsensusLoggerFactory');
const CreditsDistributionPool = require('./creditsDistributionPool/CreditsDistributionPool');
const closeAbciServerFactory = require('./abci/closeAbciServerFactory');
const getLatestFeatureFlagFactory = require('./featureFlag/getLatestFeatureFlagFactory');
const getFeatureFlagForHeightFactory = require('./featureFlag/getFeatureFlagForHeightFactory');
const ValidatorSet = require('./validator/ValidatorSet');
const createValidatorSetUpdate = require('./abci/handlers/validator/createValidatorSetUpdate');
const fetchQuorumMembersFactory = require('./core/fetchQuorumMembersFactory');
const getRandomQuorum = require('./core/getRandomQuorum');
const createQueryResponseFactory = require('./abci/handlers/query/response/createQueryResponseFactory');
const BlockExecutionContextRepository = require('./blockExecution/BlockExecutionContextRepository');
const registerSystemDataContractFactory = require('./state/registerSystemDataContractFactory');
const registerTopLevelDomainFactory = require('./state/registerTopLevelDomainFactory');
const registerFeatureFlagFactory = require('./state/registerFeatureFlagFactory');

/**
 *
 * @param {Object} options
 * @param {string} options.ABCI_HOST
 * @param {string} options.ABCI_PORT
 * @param {string} options.COMMON_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_COMMON_STORE_MERK_DB_FILE
 * @param {string} options.DATA_CONTRACT_CACHE_SIZE
 * @param {string} options.IDENTITIES_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_IDENTITIES_STORE_MERK_DB_FILE
 * @param {string} options.PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE
 * @param {string} options.DATA_CONTRACTS_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_DATA_CONTRACTS_STORE_MERK_DB_FILE
 * @param {string} options.DOCUMENTS_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_DOCUMENTS_STORE_MERK_DB_FILE
 * @param {string} options.EXTERNAL_STORE_LEVEL_DB_FILE
 * @param {string} options.PREVIOUS_EXTERNAL_STORE_LEVEL_DB_FILE
 * @param {string} options.DOCUMENT_MONGODB_DB_PREFIX
 * @param {string} options.PREVIOUS_DOCUMENT_MONGODB_DB_PREFIX
 * @param {string} options.DOCUMENT_MONGODB_URL
 * @param {string} options.ASSET_LOCK_TRANSACTIONS_STORE_MERK_DB_FILE
 * @param {string} options.PREVIOUS_ASSET_LOCK_TRANSACTIONS_STORE_MERK_DB_FILE
 * @param {string} options.CORE_JSON_RPC_HOST
 * @param {string} options.CORE_JSON_RPC_PORT
 * @param {string} options.CORE_JSON_RPC_USERNAME
 * @param {string} options.CORE_JSON_RPC_PASSWORD
 * @param {string} options.CORE_ZMQ_HOST
 * @param {string} options.CORE_ZMQ_PORT
 * @param {string} options.CORE_ZMQ_CONNECTION_RETRIES
 * @param {string} options.PREVIOUS_BLOCK_EXECUTION_TRANSACTIONS_FILE
 * @param {string} options.NETWORK
 * @param {string} options.DPNS_MASTER_PUBLIC_KEY
 * @param {string} options.DASHPAY_MASTER_PUBLIC_KEY
 * @param {string} options.FEATURE_FLAGS_MASTER_PUBLIC_KEY
 * @param {string} options.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY
 * @param {string} options.INITIAL_CORE_CHAINLOCKED_HEIGHT
 * @param {string} options.VALIDATOR_SET_LLMQ_TYPE
 * @param {string} options.LOG_STDOUT_LEVEL
 * @param {string} options.LOG_PRETTY_FILE_LEVEL
 * @param {string} options.LOG_PRETTY_FILE_PATH
 * @param {string} options.LOG_JSON_FILE_LEVEL
 * @param {string} options.LOG_JSON_FILE_PATH
 * @param {string} options.LOG_STATE_REPOSITORY
 * @param {string} options.LOG_MERK
 * @param {string} options.NODE_ENV
 *
 * @return {AwilixContainer}
 */
function createDIContainer(options) {
  if (!options.DPNS_MASTER_PUBLIC_KEY) {
    throw new Error('DPNS_MASTER_PUBLIC_KEY must be set');
  }

  if (!options.DASHPAY_MASTER_PUBLIC_KEY) {
    throw new Error('DASHPAY_MASTER_PUBLIC_KEY must be set');
  }

  if (!options.FEATURE_FLAGS_MASTER_PUBLIC_KEY) {
    throw new Error('FEATURE_FLAGS_MASTER_PUBLIC_KEY must be set');
  }

  if (!options.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY) {
    throw new Error('MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY must be set');
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
   * Register latest protocol version
   * Define highest supported protocol version
   */
  container.register({
    latestProtocolVersion: asValue(Long.fromInt(protocolVersion.latestVersion)),
  });

  /**
   * Register environment variables
   */
  container.register({
    abciHost: asValue(options.ABCI_HOST),
    abciPort: asValue(options.ABCI_PORT),
    commonStoreMerkDBFile: asValue(options.COMMON_STORE_MERK_DB_FILE),
    previousCommonStoreMerkDBFile: asValue(options.PREVIOUS_COMMON_STORE_MERK_DB_FILE),
    dataContractCacheSize: asValue(options.DATA_CONTRACT_CACHE_SIZE),
    publicKeyToIdentityIdStoreMerkDBFile:
      asValue(options.PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE),
    previousPublicKeyToIdentityIdStoreMerkDBFile:
      asValue(options.PREVIOUS_PUBLIC_KEY_TO_IDENTITY_STORE_MERK_DB_FILE),
    identitiesStoreMerkDBFile: asValue(options.IDENTITIES_STORE_MERK_DB_FILE),
    previousIdentitiesStoreMerkDBFile: asValue(options.PREVIOUS_IDENTITIES_STORE_MERK_DB_FILE),
    externalStoreLevelDBFile: asValue(options.EXTERNAL_STORE_LEVEL_DB_FILE),
    previousExternalStoreLevelDBFile: asValue(options.PREVIOUS_EXTERNAL_STORE_LEVEL_DB_FILE),
    dataContractsStoreMerkDBFile: asValue(options.DATA_CONTRACTS_STORE_MERK_DB_FILE),
    previousDataContractsStoreMerkDBFile:
      asValue(options.PREVIOUS_DATA_CONTRACTS_STORE_MERK_DB_FILE),
    documentsStoreMerkDBFile: asValue(options.DOCUMENTS_STORE_MERK_DB_FILE),
    previousDocumentsStoreMerkDBFile: asValue(options.PREVIOUS_DOCUMENTS_STORE_MERK_DB_FILE),
    documentMongoDBPrefix: asValue(options.DOCUMENT_MONGODB_DB_PREFIX),
    previousDocumentMongoDBPrefix: asValue(options.PREVIOUS_DOCUMENT_MONGODB_DB_PREFIX),
    documentMongoDBUrl: asValue(options.DOCUMENT_MONGODB_URL),
    spentAssetLockTransactionsStoreMerkDBFile:
      asValue(options.ASSET_LOCK_TRANSACTIONS_STORE_MERK_DB_FILE),
    previousSpentAssetLockTransactionsStoreMerkDBFile:
      asValue(options.PREVIOUS_ASSET_LOCK_TRANSACTIONS_STORE_MERK_DB_FILE),
    coreJsonRpcHost: asValue(options.CORE_JSON_RPC_HOST),
    coreJsonRpcPort: asValue(options.CORE_JSON_RPC_PORT),
    coreJsonRpcUsername: asValue(options.CORE_JSON_RPC_USERNAME),
    coreJsonRpcPassword: asValue(options.CORE_JSON_RPC_PASSWORD),
    coreZMQHost: asValue(options.CORE_ZMQ_HOST),
    coreZMQPort: asValue(options.CORE_ZMQ_PORT),
    coreZMQConnectionRetries: asValue(
      parseInt(options.CORE_ZMQ_CONNECTION_RETRIES, 10),
    ),
    previousBlockExecutionTransactionFile: asValue(
      options.PREVIOUS_BLOCK_EXECUTION_TRANSACTIONS_FILE,
    ),
    network: asValue(options.NETWORK),
    logStdoutLevel: asValue(options.LOG_STDOUT_LEVEL),
    logPrettyFileLevel: asValue(options.LOG_PRETTY_FILE_LEVEL),
    logPrettyFilePath: asValue(options.LOG_PRETTY_FILE_PATH),
    logJsonFileLevel: asValue(options.LOG_JSON_FILE_LEVEL),
    logJsonFilePath: asValue(options.LOG_JSON_FILE_PATH),
    logStateRepository: asValue(options.LOG_STATE_REPOSITORY === 'true'),
    logMerk: asValue(options.LOG_MERK === 'true'),
    isProductionEnvironment: asValue(options.NODE_ENV === 'production'),
    maxIdentitiesPerRequest: asValue(25),
    smlMaxListsLimit: asValue(16),
    initialCoreChainLockedHeight: asValue(
      parseInt(options.INITIAL_CORE_CHAINLOCKED_HEIGHT, 10),
    ),
    validatorSetLLMQType: asValue(
      parseInt(options.VALIDATOR_SET_LLMQ_TYPE, 10),
    ),
    masternodeRewardSharesContractId: asValue(
      Identifier.from(masternodeRewardsSystemIds.contractId),
    ),
    masternodeRewardSharesOwnerId: asValue(
      Identifier.from(masternodeRewardsSystemIds.ownerId),
    ),
    masternodeRewardSharesOwnerPublicKey: asValue(
      PublicKey.fromString(
        options.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY,
      ),
    ),
    masternodeRewardSharesDocuments: asValue(
      masternodeRewardsDocuments,
    ),
    featureFlagsContractId: asValue(
      Identifier.from(featureFlagsSystemIds.contractId),
    ),
    featureFlagsOwnerId: asValue(
      Identifier.from(featureFlagsSystemIds.ownerId),
    ),
    featureFlagsOwnerPublicKey: asValue(
      PublicKey.fromString(
        options.FEATURE_FLAGS_MASTER_PUBLIC_KEY,
      ),
    ),
    featureFlagsDocuments: asValue(featureFlagsDocuments),
    dpnsContractId: asValue(Identifier.from(dpnsSystemIds.contractId)),
    dpnsOwnerId: asValue(Identifier.from(dpnsSystemIds.ownerId)),
    dpnsOwnerPublicKey: asValue(
      PublicKey.fromString(
        options.DPNS_MASTER_PUBLIC_KEY,
      ),
    ),
    dpnsDocuments: asValue(dpnsDocuments),
    dashpayContractId: asValue(Identifier.from(dashpaySystemIds.contractId)),
    dashpayOwnerId: asValue(Identifier.from(dashpaySystemIds.ownerId)),
    dashpayOwnerPublicKey: asValue(
      PublicKey.fromString(
        options.DASHPAY_MASTER_PUBLIC_KEY,
      ),
    ),
    dashpayDocuments: asValue(dashpayDocuments),
    tenderdashP2pPort: asValue(options.TENDERDASH_P2P_PORT),
  });

  /**
   * Register global DPP options
   */
  container.register({
    dppOptions: asValue({}),
  });

  /**
   * Register Core related
   */
  container.register({
    latestCoreChainLock: asValue(new LatestCoreChainLock()),
    simplifiedMasternodeList: asClass(SimplifiedMasternodeList).proxy().singleton(),
    decodeChainLock: asValue(decodeChainLock),
    fetchQuorumMembers: asFunction(fetchQuorumMembersFactory),
    getRandomQuorum: asValue(getRandomQuorum),
  });

  /**
   * Register common services
   */
  container.register({
    loggerPrettyfierOptions: asValue({
      translateTime: true,
    }),

    logStdoutStream: asFunction((loggerPrettyfierOptions) => pinoMultistream.prettyStream({
      prettyPrint: loggerPrettyfierOptions,
    })).singleton(),

    logPrettyFileStream: asFunction((
      logPrettyFilePath,
      loggerPrettyfierOptions,
    ) => pinoMultistream.prettyStream({
      prettyPrint: loggerPrettyfierOptions,
      dest: fs.createWriteStream(logPrettyFilePath, { flags: 'a' }),
    })).singleton(),

    logJsonFileStream: asFunction((logJsonFilePath) => fs.createWriteStream(logJsonFilePath, { flags: 'a' }))
      .disposer(async (stream) => new Promise((resolve) => {
        stream.end(resolve);
      })).singleton(),

    loggerStreams: asFunction((
      logStdoutLevel,
      logStdoutStream,
      logPrettyFileLevel,
      logPrettyFileStream,
      logJsonFileLevel,
      logJsonFileStream,
    ) => [
      {
        level: logStdoutLevel,
        stream: logStdoutStream,
      },
      {
        level: logPrettyFileLevel,
        stream: logPrettyFileStream,
      },
      {
        level: logJsonFileLevel,
        stream: logJsonFileStream,
      },
    ]),

    logger: asFunction(
      (loggerStreams) => pino({
        level: 'trace',
      }, pinoMultistream.multistream(loggerStreams))
        .child({ driveVersion: packageJSON.version }),
    ).singleton(),

    noopLogger: asFunction(() => (
      Object.keys(pino.levels.values).reduce((logger, functionName) => ({
        ...logger,
        [functionName]: () => {},
      }), {})
    )).singleton(),

    sanitizeUrl: asValue(sanitizeUrl),
    coreZMQClient: asFunction((
      coreZMQHost,
      coreZMQPort,
      coreZMQConnectionRetries,
    ) => (
      new ZMQClient(coreZMQHost, coreZMQPort, {
        maxRetryCount: coreZMQConnectionRetries,
      })
    )).singleton(),

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
      })
    )).singleton(),
  });

  /**
   * Register Identity
   */
  container.register({
    publicKeyToIdentityIdStore: asFunction((
      publicKeyToIdentityIdStoreMerkDBFile,
      logger,
      logMerk,
    ) => {
      const merkDb = new Merk(publicKeyToIdentityIdStoreMerkDBFile);

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'publicKeyToIdentityId');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousPublicKeyToIdentityIdStore: asFunction((
      previousPublicKeyToIdentityIdStoreMerkDBFile,
    ) => {
      const merkDb = new Merk(previousPublicKeyToIdentityIdStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    publicKeyToIdentityIdStoreRootTreeLeaf: asClass(PublicKeyToIdentityIdStoreRootTreeLeaf)
      .singleton(),

    previousPublicKeyToIdentityIdStoreRootTreeLeaf: asFunction((
      previousPublicKeyToIdentityIdStore,
    ) => (new PublicKeyToIdentityIdStoreRootTreeLeaf(previousPublicKeyToIdentityIdStore))),

    identitiesStore: asFunction((identitiesStoreMerkDBFile, logger, logMerk) => {
      const merkDb = new Merk(identitiesStoreMerkDBFile);

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'identities');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousIdentitiesStore: asFunction((previousIdentitiesStoreMerkDBFile) => {
      const merkDb = new Merk(previousIdentitiesStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    identitiesStoreRootTreeLeaf: asClass(IdentitiesStoreRootTreeLeaf).singleton(),
    previousIdentitiesStoreRootTreeLeaf: asFunction((
      previousIdentitiesStore,
    ) => (new IdentitiesStoreRootTreeLeaf(previousIdentitiesStore))).singleton(),

    identityRepository: asClass(IdentityStoreRepository).singleton(),
    previousIdentityRepository: asFunction((
      previousIdentitiesStore,
    ) => (new IdentityStoreRepository(previousIdentitiesStore, container))).singleton(),

    publicKeyToIdentityIdRepository: asClass(PublicKeyToIdentityIdStoreRepository).singleton(),
    previousPublicKeyToIdentityIdRepository: asFunction((
      previousPublicKeyToIdentityIdStore,
    ) => (
      new PublicKeyToIdentityIdStoreRepository(previousPublicKeyToIdentityIdStore)
    )).singleton(),
  });

  /**
   * Register asset lock transactions
   */
  container.register({
    spentAssetLockTransactionsStore: asFunction((
      spentAssetLockTransactionsStoreMerkDBFile,
      logger,
      logMerk,
    ) => {
      const merkDb = new Merk(
        spentAssetLockTransactionsStoreMerkDBFile,
      );

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'spentAssetLockTransactions');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousSpentAssetLockTransactionsStore: asFunction((
      previousSpentAssetLockTransactionsStoreMerkDBFile,
    ) => {
      const merkDb = new Merk(
        previousSpentAssetLockTransactionsStoreMerkDBFile,
      );

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    spentAssetLockTransactionsRepository: asClass(SpentAssetLockTransactionsRepository).singleton(),

    previousSpentAssetLockTransactionsRepository: asFunction((
      previousSpentAssetLockTransactionsStore,
    ) => (
      new SpentAssetLockTransactionsRepository(previousSpentAssetLockTransactionsStore)
    )).singleton(),

    spentAssetLockTransactionsStoreRootTreeLeaf: asClass(
      SpentAssetLockTransactionsStoreRootTreeLeaf,
    ).singleton(),

    previousSpentAssetLockTransactionsStoreRootTreeLeaf: asFunction((
      previousSpentAssetLockTransactionsStore,
    ) => (new SpentAssetLockTransactionsStoreRootTreeLeaf(
      previousSpentAssetLockTransactionsStore,
    ))).singleton(),
  });

  /**
   * Register Data Contract
   */
  container.register({
    dataContractsStore: asFunction((dataContractsStoreMerkDBFile, logger, logMerk) => {
      const merkDb = new Merk(dataContractsStoreMerkDBFile);

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'dataContracts');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousDataContractsStore: asFunction((previousDataContractsStoreMerkDBFile) => {
      const merkDb = new Merk(previousDataContractsStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    dataContractsStoreRootTreeLeaf: asClass(DataContractsStoreRootTreeLeaf).singleton(),

    previousDataContractsStoreRootTreeLeaf: asFunction((
      previousDataContractsStore,
    ) => (new DataContractsStoreRootTreeLeaf(previousDataContractsStore))).singleton(),

    dataContractRepository: asClass(DataContractStoreRepository).singleton(),

    previousDataContractRepository: asFunction((
      previousDataContractsStore,
    ) => (new DataContractStoreRepository(previousDataContractsStore, container))).singleton(),

    dataContractCache: asFunction((dataContractCacheSize) => (
      new LRUCache(dataContractCacheSize)
    )).singleton(),

    previousDataContractCache: asFunction((dataContractCacheSize) => (
      new LRUCache(dataContractCacheSize)
    )).singleton(),
  });

  /**
   * Register Document
   */
  container.register({
    documentsStore: asFunction((documentsStoreMerkDBFile, logger, logMerk) => {
      const merkDb = new Merk(documentsStoreMerkDBFile);

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'documents');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousDocumentsStore: asFunction((previousDocumentsStoreMerkDBFile) => {
      const merkDb = new Merk(previousDocumentsStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    documentStoreRepository: asClass(DocumentStoreRepository).singleton(),
    previousDocumentStoreRepository: asFunction((
      previousDocumentsStore,
    ) => (new DocumentStoreRepository(previousDocumentsStore, container))).singleton(),

    documentsStoreRootTreeLeaf: asClass(DocumentsStoreRootTreeLeaf).singleton(),
    previousDocumentsStoreRootTreeLeaf: asFunction((
      previousDocumentsStore,
    ) => (new DocumentsStoreRootTreeLeaf(previousDocumentsStore))).singleton(),

    documentRepository: asClass(DocumentIndexedStoreRepository).singleton(),

    previousDocumentRepository: asFunction((
      previousDocumentStoreRepository,
      createPreviousDocumentMongoDbRepository,
    ) => (
      new DocumentIndexedStoreRepository(
        previousDocumentStoreRepository,
        createPreviousDocumentMongoDbRepository,
      )
    )).singleton(),

    connectToDocumentMongoDB: asFunction((documentMongoDBUrl) => (
      connectToMongoDBFactory(documentMongoDBUrl)
    )).singleton(),

    findConflictingConditions: asValue(findConflictingConditions),
    convertWhereToMongoDbQuery: asValue(convertWhereToMongoDbQuery),
    validateQuery: asFunction(validateQueryFactory).singleton(),

    convertToMongoDbIndices: asValue(convertToMongoDbIndicesFunction),

    getDocumentMongoDBDatabase: asFunction(getDocumentDatabaseFactory).singleton(),

    getPreviousDocumentMongoDBDatabase: asFunction((
      connectToDocumentMongoDB,
      previousDocumentMongoDBPrefix,
    ) => (
      getDocumentDatabaseFactory(
        connectToDocumentMongoDB,
        previousDocumentMongoDBPrefix,
      )
    )).singleton(),

    createDocumentMongoDbRepository: asFunction(createDocumentMongoDbRepositoryFactory).singleton(),

    createPreviousDocumentMongoDbRepository: asFunction((
      validateQuery,
      getPreviousDocumentMongoDBDatabase,
      dataContractRepository,
    ) => (
      createDocumentMongoDbRepositoryFactory(
        convertWhereToMongoDbQuery,
        validateQuery,
        getPreviousDocumentMongoDBDatabase,
        dataContractRepository,
        container,
        { isPrevious: true },
      )
    )).singleton(),

    documentDatabaseManager: asClass(DocumentDatabaseManager).singleton(),

    previousDocumentDatabaseManager: asFunction((
      createPreviousDocumentMongoDbRepository,
      convertToMongoDbIndices,
      getPreviousDocumentMongoDBDatabase,
    ) => (
      new DocumentDatabaseManager(
        createPreviousDocumentMongoDbRepository,
        convertToMongoDbIndices,
        getPreviousDocumentMongoDBDatabase,
      )
    )).singleton(),

    fetchDocuments: asFunction(fetchDocumentsFactory).singleton(),

    fetchPreviousDocuments: asFunction((
      previousDocumentRepository,
      previousDataContractRepository,
      previousDataContractCache,
    ) => (
      fetchDocumentsFactory(
        previousDocumentRepository,
        previousDataContractRepository,
        previousDataContractCache,
      )
    )).singleton(),

    populateMongoDbTransactionFromObject: asFunction((
      createPreviousDocumentMongoDbRepository,
      dpp,
    ) => (
      populateMongoDbTransactionFromObjectFactory(
        createPreviousDocumentMongoDbRepository,
        dpp,
      )
    )).singleton(),
  });

  /**
   * Register chain info
   * */
  container.register({
    externalLevelDB: asFunction((externalStoreLevelDBFile) => (
      level(externalStoreLevelDBFile, { keyEncoding: 'binary', valueEncoding: 'binary' })
    )).disposer((levelDB) => levelDB.close())
      .singleton(),

    previousExternalLevelDB: asFunction((previousExternalStoreLevelDBFile) => (
      level(previousExternalStoreLevelDBFile, { keyEncoding: 'binary', valueEncoding: 'binary' })
    )).disposer((levelDB) => levelDB.close())
      .singleton(),
  });

  /**
   * Register credits distribution pool
   */
  container.register({
    commonStore: asFunction((commonStoreMerkDBFile, logger, logMerk) => {
      const merkDb = new Merk(commonStoreMerkDBFile);

      return new MerkDbStore(merkDb, logMerk ? logger : null, 'common');
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    previousCommonStore: asFunction((previousCommonStoreMerkDBFile) => {
      const merkDb = new Merk(previousCommonStoreMerkDBFile);

      return new MerkDbStore(merkDb);
    }).disposer((merkDb) => {
      // Flush data on disk
      merkDb.db.flushSync();

      // Drop test database
      if (process.env.NODE_ENV === 'test') {
        merkDb.db.destroy();
      } else {
        // or close in case of production
        merkDb.db.close();
      }
    }).singleton(),

    commonStoreRootTreeLeaf: asClass(CommonStoreRootTreeLeaf).singleton(),
    previousCommonStoreRootTreeLeaf: asFunction((
      previousCommonStore,
    ) => (new CommonStoreRootTreeLeaf(previousCommonStore))).singleton(),

    creditsDistributionPoolRepository: asClass(CreditsDistributionPoolCommonStoreRepository)
      .singleton(),

    previousCreditsDistributionPoolRepository: asFunction((
      previousCommonStore,
    ) => (new CreditsDistributionPoolCommonStoreRepository(previousCommonStore))).singleton(),

    creditsDistributionPool: asValue(new CreditsDistributionPool()),
  });

  /**
   * Register block execution context
   */
  container.register({
    blockExecutionStoreTransactions: asClass(BlockExecutionStoreTransactions).singleton(),

    previousBlockExecutionStoreTransactionsRepository: asClass(
      PreviousBlockExecutionStoreTransactionsRepository,
    ).singleton(),

    previousBlockExecutionTransactionDB: asFunction((previousBlockExecutionTransactionFile) => (
      new FileDb(previousBlockExecutionTransactionFile)
    )).singleton(),

    blockExecutionContext: asClass(BlockExecutionContext).singleton(),

    previousBlockExecutionContext: asClass(BlockExecutionContext).singleton(),

    blockExecutionContextRepository: asClass(BlockExecutionContextRepository).singleton(),

    cloneToPreviousStoreTransactions: asFunction(
      cloneToPreviousStoreTransactionsFactory,
    ).singleton(),
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
      spentAssetLockTransactionsStoreRootTreeLeaf,
    ) => (
      new RootTree([
        commonStoreRootTreeLeaf,
        identitiesStoreRootTreeLeaf,
        publicKeyToIdentityIdStoreRootTreeLeaf,
        dataContractsStoreRootTreeLeaf,
        documentsStoreRootTreeLeaf,
        spentAssetLockTransactionsStoreRootTreeLeaf,
      ])
    )).singleton(),

    previousRootTree: asFunction((
      previousCommonStoreRootTreeLeaf,
      previousIdentitiesStoreRootTreeLeaf,
      previousPublicKeyToIdentityIdStoreRootTreeLeaf,
      previousDataContractsStoreRootTreeLeaf,
      previousDocumentsStoreRootTreeLeaf,
      previousSpentAssetLockTransactionsStoreRootTreeLeaf,
    ) => (
      new RootTree([
        previousCommonStoreRootTreeLeaf,
        previousIdentitiesStoreRootTreeLeaf,
        previousPublicKeyToIdentityIdStoreRootTreeLeaf,
        previousDataContractsStoreRootTreeLeaf,
        previousDocumentsStoreRootTreeLeaf,
        previousSpentAssetLockTransactionsStoreRootTreeLeaf,
      ])
    )).singleton(),
  });

  /**
   * Register DPP
   */
  container.register({
    stateRepository: asFunction((
      identityRepository,
      publicKeyToIdentityIdRepository,
      dataContractRepository,
      fetchDocuments,
      documentRepository,
      spentAssetLockTransactionsRepository,
      coreRpcClient,
      dataContractCache,
      blockExecutionContext,
      simplifiedMasternodeList,
      getLatestFeatureFlag,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyToIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        documentRepository,
        spentAssetLockTransactionsRepository,
        coreRpcClient,
        blockExecutionContext,
        simplifiedMasternodeList,
        getLatestFeatureFlag,
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
      spentAssetLockTransactionsRepository,
      coreRpcClient,
      blockExecutionStoreTransactions,
      dataContractCache,
      blockExecutionContext,
      simplifiedMasternodeList,
      logStateRepository,
      getLatestFeatureFlag,
    ) => {
      const stateRepository = new DriveStateRepository(
        identityRepository,
        publicKeyToIdentityIdRepository,
        dataContractRepository,
        fetchDocuments,
        documentRepository,
        spentAssetLockTransactionsRepository,
        coreRpcClient,
        blockExecutionContext,
        simplifiedMasternodeList,
        getLatestFeatureFlag,
        blockExecutionStoreTransactions,
      );

      const cachedRepository = new CachedStateRepositoryDecorator(
        stateRepository,
        dataContractCache,
      );

      if (!logStateRepository) {
        return cachedRepository;
      }

      return new LoggedStateRepositoryDecorator(
        cachedRepository,
        blockExecutionContext,
      );
    }).singleton(),

    unserializeStateTransition: asFunction((
      dpp,
      noopLogger,
    ) => unserializeStateTransitionFactory(dpp, noopLogger)).singleton(),

    transactionalUnserializeStateTransition: asFunction((
      transactionalDpp,
      noopLogger,
    ) => unserializeStateTransitionFactory(transactionalDpp, noopLogger)).singleton(),

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
   * Register validator quorums
   */
  container.register({
    validatorSet: asClass(ValidatorSet),
  });

  /**
   * Register feature flags stuff
   */
  container.register({
    getLatestFeatureFlag: asFunction(getLatestFeatureFlagFactory),
    getFeatureFlagForHeight: asFunction(getFeatureFlagForHeightFactory),
  });

  /**
   * Register Core stuff
   */
  container.register({
    waitForCoreSync: asFunction(waitForCoreSyncFactory).singleton(),

    updateSimplifiedMasternodeList: asFunction(updateSimplifiedMasternodeListFactory).singleton(),

    waitForChainLockedHeight: asFunction(waitForChainLockedHeightFactory).singleton(),

    waitForCoreChainLockSync: asFunction(waitForCoreChainLockSyncFactory).singleton(),

    waitReplicaSetInitialize: asFunction(waitReplicaSetInitializeFactory).singleton(),
  });

  /**
   * Register system data contract utils
   */
  container.register({
    registerSystemDataContract: asFunction(registerSystemDataContractFactory).singleton(),
    registerTopLevelDomain: asFunction(registerTopLevelDomainFactory).singleton(),
    registerFeatureFlag: asFunction(registerFeatureFlagFactory).singleton(),
    cumulativeFeesFeatureFlagDocumentId: asValue(
      Identifier.from('73qjFBuY4Zb8DqUpj6RYgG9rYDARNPFnEnyt8u4bxPcw'),
    ),
    dashPreorderDocumentId: asValue(
      Identifier.from('i8QZtAJ1WshunyZg64wGYcm3jASrpeSKAbAYVHTxvsL'),
    ),
    dashDomainDocumentId: asValue(
      Identifier.from('FXyN2NZAdRFADgBQfb1XM1Qq7pWoEcgSWj1GaiQJqcrS'),
    ),
    dashPreorderSalt: asValue(
      Buffer.from('e0b508c5a36825a206693a1f414aa13edbecf43c41e3c799ea9e737b4f9aa226', 'hex'),
    ),
  });

  /**
   * Register ABCI handlers
   */
  container.register({
    createQueryResponse: asFunction(createQueryResponseFactory).singleton(),
    createValidatorSetUpdate: asValue(createValidatorSetUpdate),
    identityQueryHandler: asFunction(identityQueryHandlerFactory).singleton(),
    dataContractQueryHandler: asFunction(dataContractQueryHandlerFactory).singleton(),
    documentQueryHandler: asFunction(documentQueryHandlerFactory).singleton(),
    getProofsQueryHandler: asFunction(getProofsQueryHandlerFactory).singleton(),
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
      getProofsQueryHandler,
    ) => {
      const router = findMyWay({
        ignoreTrailingSlash: true,
      });

      router.on('GET', '/identities', identityQueryHandler);
      router.on('GET', '/dataContracts', dataContractQueryHandler);
      router.on('GET', '/dataContracts/documents', documentQueryHandler);
      router.on('GET', '/proofs', getProofsQueryHandler);
      router.on('GET', '/identities/by-public-key-hash', identitiesByPublicKeyHashesQueryHandler);
      router.on('GET', '/identities/by-public-key-hash/id', identityIdsByPublicKeyHashesQueryHandler);
      router.on('GET', '/verify-chainlock', verifyChainLockQueryHandler, { rawData: true });

      return router;
    }).singleton(),

    infoHandler: asFunction(infoHandlerFactory).singleton(),
    checkTxHandler: asFunction(checkTxHandlerFactory).singleton(),
    beginBlockHandler: asFunction(beginBlockHandlerFactory).singleton(),
    deliverTxHandler: asFunction(deliverTxHandlerFactory).singleton(),
    initChainHandler: asFunction(initChainHandlerFactory).singleton(),
    endBlockHandler: asFunction(endBlockHandlerFactory).singleton(),
    commitHandler: asFunction(commitHandlerFactory).singleton(),
    queryHandler: asFunction(queryHandlerFactory).singleton(),

    wrapInErrorHandler: asFunction(wrapInErrorHandlerFactory).singleton(),
    enrichErrorWithConsensusError: asFunction(enrichErrorWithConsensusErrorFactory).singleton(),
    errorHandler: asFunction(errorHandlerFactory).singleton(),

    abciHandlers: asFunction((
      infoHandler,
      checkTxHandler,
      beginBlockHandler,
      deliverTxHandler,
      initChainHandler,
      endBlockHandler,
      commitHandler,
      wrapInErrorHandler,
      enrichErrorWithConsensusError,
      queryHandler,
    ) => ({
      info: infoHandler,
      checkTx: wrapInErrorHandler(checkTxHandler, { respondWithInternalError: true }),
      beginBlock: enrichErrorWithConsensusError(beginBlockHandler),
      deliverTx: wrapInErrorHandler(enrichErrorWithConsensusError(deliverTxHandler)),
      initChain: initChainHandler,
      endBlock: enrichErrorWithConsensusError(endBlockHandler),
      commit: enrichErrorWithConsensusError(commitHandler),
      query: wrapInErrorHandler(queryHandler, { respondWithInternalError: true }),
    })).singleton(),

    closeAbciServer: asFunction(closeAbciServerFactory).singleton(),

    abciServer: asFunction((abciHandlers) => createABCIServer(abciHandlers))
      .singleton(),
  });

  return container;
}

module.exports = createDIContainer;

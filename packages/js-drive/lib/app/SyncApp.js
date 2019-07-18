const IpfsAPI = require('ipfs-http-client');
const RpcClient = require('@dashevo/dashd-rpc/promise');
const { MongoClient } = require('mongodb');
const DashPlatformProtocol = require('@dashevo/dpp');

const DriveDataProvider = require('../dpp/DriveDataProvider');

const Logger = require('../util/Logger');

const SyncStateRepository = require('../sync/state/repository/SyncStateRepository');
const SVContractMongoDbRepository = require('../stateView/contract/SVContractMongoDbRepository');
const SVDocumentMongoDbRepository = require('../stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const createSVDocumentMongoDbRepositoryFactory = require('../stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const convertWhereToMongoDbQuery = require('../stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../stateView/document/query/findConflictingConditions');
const updateSVContractFactory = require('../stateView/contract/updateSVContractFactory');
const updateSVDocumentFactory = require('../stateView/document/updateSVDocumentFactory');
const revertSVDocumentsForStateTransitionFactory = require('../stateView/document/revertSVDocumentsForStateTransitionFactory');
const revertSVContractsForStateTransitionFactory = require('../stateView/contract/revertSVContractsForStateTransitionFactory');
const applyStateTransitionFactory = require('../stateView/applyStateTransitionFactory');
const applyStateTransitionFromReferenceFactory = require('../stateView/applyStateTransitionFromReferenceFactory');
const BlockchainReader = require('../blockchain/reader/BlockchainReader');
const RpcBlockIterator = require('../blockchain/blockIterator/RpcBlockIterator');
const BlockchainReaderState = require('../blockchain/reader/BlockchainReaderState');
const BlockchainReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');
const createStateTransitionsFromBlockFactory = require('../blockchain/createStateTransitionsFromBlockFactory');
const readBlockchainFactory = require('../blockchain/reader/readBlockchainFactory');

const fetchContractFactory = require('../stateView/contract/fetchContractFactory');
const fetchDocumentsFactory = require('../stateView/document/fetchDocumentsFactory');

const STPacketIpfsRepository = require('../storage/stPacket/STPacketIpfsRepository');

const dropMongoDatabasesWithPrefixFactory = require('../mongoDb/dropMongoDatabasesWithPrefixFactory');

const isDashCoreRunningFactory = require('../sync/isDashCoreRunningFactory');
const DashCoreIsNotRunningError = require('../sync/DashCoreIsNotRunningError');

class SyncApp {
  /**
   * @param {SyncAppOptions} options
   */
  constructor(options) {
    this.options = options;
    this.rpcClient = new RpcClient({
      protocol: 'http',
      host: this.options.getDashCoreJsonRpcHost(),
      port: this.options.getDashCoreJsonRpcPort(),
      user: this.options.getDashCoreJsonRpcUser(),
      pass: this.options.getDashCoreJsonRpcPass(),
    });
    this.ipfsAPI = new IpfsAPI(this.options.getStorageIpfsMultiAddr());
    this.mongoClient = null;
    this.syncStateRepository = null;
    this.syncState = null;
    this.blockchainReaderMediator = null;
    this.blockchainReaderState = null;
    this.stateTransitionsFromBlock = null;
  }

  /**
   * Init SyncApp
   *
   * @returns {Promise<void>}
   */
  async init() {
    const isDashCoreRunning = isDashCoreRunningFactory(this.rpcClient);

    const isRunning = await isDashCoreRunning(
      this.options.getDashCoreRunningCheckMaxRetries(),
      this.options.getDashCoreRunningCheckInterval(),
    );
    if (!isRunning) {
      throw new DashCoreIsNotRunningError();
    }

    this.mongoClient = await MongoClient.connect(
      this.options.getStorageMongoDbUrl(),
      { useNewUrlParser: true },
    );

    const mongoDb = this.mongoClient.db(this.options.getStorageMongoDbDatabase());
    this.syncStateRepository = new SyncStateRepository(mongoDb);
    this.syncState = await this.syncStateRepository.fetch();
  }

  /**
   * Get Mongo client
   *
   * @returns {MongoClient}
   */
  getMongoClient() {
    return this.mongoClient;
  }

  /**
   * Get RPC client
   *
   * @returns {RpcClient}
   */
  getRpcClient() {
    return this.rpcClient;
  }

  /**
   * Get syncStateRepository
   *
   * @returns {SyncStateRepository}
   */
  getSyncStateRepository() {
    return this.syncStateRepository;
  }

  /**
   * Get SyncState
   *
   * @returns {SyncState}
   */
  getSyncState() {
    return this.syncState;
  }

  /**
   * Get ipfsAPI
   *
   * @returns {IpfsAPI}
   */
  getIpfsApi() {
    return this.ipfsAPI;
  }

  /**
   * @return {Logger}
   */
  createLogger() {
    return new Logger(console);
  }

  /**
   * @return {BlockchainReaderState}
   */
  createBlockchainReaderState() {
    if (!this.blockchainReaderState) {
      this.blockchainReaderState = new BlockchainReaderState(
        this.getSyncState().getBlocks(),
        this.options.getSyncStateBlocksLimit(),
      );
    }

    return this.blockchainReaderState;
  }

  /**
   * @return {BlockchainReaderMediator}
   */
  createBlockchainReaderMediator() {
    if (!this.blockchainReaderMediator) {
      this.blockchainReaderMediator = new BlockchainReaderMediator(
        this.createBlockchainReaderState(),
        this.options.getSyncEvoStartBlockHeight(),
      );
    }

    return this.blockchainReaderMediator;
  }

  /**
   * @return {createStateTransitionsFromBlock}
   */
  createStateTransitionsFromBlock() {
    if (!this.stateTransitionsFromBlock) {
      this.stateTransitionsFromBlock = createStateTransitionsFromBlockFactory(
        this.getRpcClient(),
      );
    }

    return this.stateTransitionsFromBlock;
  }

  /**
   * @return {readBlockchain}
   */
  createReadBlockchain() {
    const blockIterator = new RpcBlockIterator(this.getRpcClient());

    const readerMediator = this.createBlockchainReaderMediator();

    const blockchainReader = new BlockchainReader(
      blockIterator,
      readerMediator,
      this.createStateTransitionsFromBlock(),
    );

    return readBlockchainFactory(
      blockchainReader,
      readerMediator,
      this.getRpcClient(),
    );
  }

  /**
   * Create dropMonoDatabasesWithPrefix
   *
   * @returns {dropMongoDatabasesWithPrefix}
   */
  createDropMongoDatabasesWithPrefix() {
    return dropMongoDatabasesWithPrefixFactory(this.getMongoClient());
  }

  /**
   * @private
   * @return {fetchContract}
   */
  createFetchContract() {
    if (!this.fetchContract) {
      const mongoDb = this.mongoClient.db(this.options.getStorageMongoDbDatabase());
      const svContractMongoDbRepository = new SVContractMongoDbRepository(
        mongoDb,
        new DashPlatformProtocol(),
      );

      this.fetchContract = fetchContractFactory(svContractMongoDbRepository);
    }

    return this.fetchContract;
  }

  /**
   * @private
   * @return {createSVDocumentMongoDbRepository}
   */
  createSVDocumentMongoDbRepository() {
    if (!this.svDocumentMongoDbRepositoryFactory) {
      const validateQuery = validateQueryFactory(findConflictingConditions);

      this.svDocumentMongoDbRepositoryFactory = createSVDocumentMongoDbRepositoryFactory(
        this.mongoClient,
        SVDocumentMongoDbRepository,
        convertWhereToMongoDbQuery,
        validateQuery,
      );
    }

    return this.svDocumentMongoDbRepositoryFactory;
  }

  /**
   * @private
   * @return {fetchDocuments}
   */
  createFetchDocuments() {
    if (!this.fetchDocuments) {
      const createSVDocumentMongoDbRepository = this.createSVDocumentMongoDbRepository();

      this.fetchDocuments = fetchDocumentsFactory(createSVDocumentMongoDbRepository);
    }

    return this.fetchDocuments;
  }

  /**
   * @private
   * @return {DashPlatformProtocol}
   */
  createDashPlatformProtocol() {
    if (!this.dashPlatfromProtocol) {
      const dataProvider = new DriveDataProvider(
        this.createFetchDocuments(),
        this.createFetchContract(),
        this.rpcClient,
      );

      this.dashPlatfromProtocol = new DashPlatformProtocol({
        dataProvider,
      });
    }

    return this.dashPlatfromProtocol;
  }

  /**
   * Create applyStateTransition
   *
   * @returns {applyStateTransition}
   */
  createApplyStateTransition() {
    if (this.applyStateTransition) {
      return this.applyStateTransition;
    }

    const mongoDb = this.getMongoClient().db(this.options.getStorageMongoDbDatabase());
    const svContractMongoDbRepository = new SVContractMongoDbRepository(
      mongoDb,
      this.createDashPlatformProtocol(),
    );
    const createSVDocumentMongoDbRepository = this.createSVDocumentMongoDbRepository();

    const updateSVContract = updateSVContractFactory(svContractMongoDbRepository);
    const updateSVDocument = updateSVDocumentFactory(createSVDocumentMongoDbRepository);

    this.applyStateTransition = applyStateTransitionFactory(
      this.createSTPacketRepository(),
      updateSVContract,
      updateSVDocument,
      this.createBlockchainReaderMediator(),
    );

    return this.applyStateTransition;
  }

  /**
   * Create applyStateTransitionFromReference
   *
   * @returns {applyStateTransitionFromReference}
   */
  createApplyStateTransitionFromReference() {
    if (!this.applyStateTransitionFromReference) {
      this.applyStateTransitionFromReference = applyStateTransitionFromReferenceFactory(
        this.createApplyStateTransition(),
        this.getRpcClient(),
      );
    }
    return this.applyStateTransitionFromReference;
  }

  /**
   * Create revertSVDocumentsForStateTransition
   *
   * @returns {revertSVDocumentsForStateTransition}
   */
  createRevertSVDocumentsForStateTransition() {
    const createSVDocumentMongoDbRepository = this.createSVDocumentMongoDbRepository();

    return revertSVDocumentsForStateTransitionFactory(
      this.createSTPacketRepository(),
      this.getRpcClient(),
      createSVDocumentMongoDbRepository,
      this.createApplyStateTransition(),
      this.createApplyStateTransitionFromReference(),
      this.createBlockchainReaderMediator(),
    );
  }

  /**
   * Create revertSVContractsForStateTransition
   *
   * @returns {revertSVContractsForStateTransition}
   */
  createRevertSVContractsForStateTransition() {
    const mongoDb = this.getMongoClient().db(this.options.getStorageMongoDbDatabase());
    const svContractMongoDbRepository = new SVContractMongoDbRepository(
      mongoDb,
      this.createDashPlatformProtocol(),
    );

    return revertSVContractsForStateTransitionFactory(
      svContractMongoDbRepository,
      this.getRpcClient(),
      this.createApplyStateTransition(),
      this.createApplyStateTransitionFromReference(),
      this.createBlockchainReaderMediator(),
    );
  }

  /**
   * Create ST Packet repository
   *
   * @return {STPacketIpfsRepository}
   */
  createSTPacketRepository() {
    if (this.stPacketRepository) {
      return this.stPacketRepository;
    }
    this.stPacketRepository = new STPacketIpfsRepository(
      this.getIpfsApi(),
      this.createDashPlatformProtocol(),
      this.options.getStorageIpfsTimeout(),
    );
    return this.stPacketRepository;
  }
}

module.exports = SyncApp;

const IpfsAPI = require('ipfs-api');
const RpcClient = require('@dashevo/dashd-rpc/promise');
const { MongoClient } = require('mongodb');

const Logger = require('../util/Logger');

const SyncStateRepository = require('../sync/state/repository/SyncStateRepository');
const sanitizeData = require('../mongoDb/sanitizeData');
const DapContractMongoDbRepository = require('../stateView/dapContract/DapContractMongoDbRepository');
const DapObjectMongoDbRepository = require('../stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const updateDapContractFactory = require('../stateView/dapContract/updateDapContractFactory');
const updateDapObjectFactory = require('../stateView/dapObject/updateDapObjectFactory');
const revertDapObjectsForStateTransitionFactory = require('../stateView/dapObject/revertDapObjectsForStateTransitionFactory');
const revertDapContractsForStateTransitionFactory = require('../stateView/dapContract/revertDapContractsForStateTransitionFactory');
const applyStateTransitionFactory = require('../stateView/applyStateTransitionFactory');
const applyStateTransitionFromReferenceFactory = require('../stateView/applyStateTransitionFromReferenceFactory');
const BlockchainReader = require('../blockchain/reader/BlockchainReader');
const RpcBlockIterator = require('../blockchain/blockIterator/RpcBlockIterator');
const BlockchainReaderState = require('../blockchain/reader/BlockchainReaderState');
const BlockchainReaderMediator = require('../blockchain/reader/BlockchainReaderMediator');
const createStateTransitionsFromBlockFactory = require('../blockchain/createStateTransitionsFromBlockFactory');
const readBlockchainFactory = require('../blockchain/reader/readBlockchainFactory');

const unpinAllIpfsPacketsFactory = require('../storage/ipfs/unpinAllIpfsPacketsFactory');
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
  // eslint-disable-next-line class-methods-use-this
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
   * Create unpinAllIpfsPackets
   *
   * @returns {unpinAllIpfsPackets}
   */
  createUnpinAllIpfsPackets() {
    return unpinAllIpfsPacketsFactory(this.getIpfsApi());
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
   * Create applyStateTransition
   *
   * @returns {applyStateTransition}
   */
  createApplyStateTransition() {
    if (this.applyStateTransition) {
      return this.applyStateTransition;
    }

    const mongoDb = this.getMongoClient().db(this.options.getStorageMongoDbDatabase());
    const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      this.getMongoClient(),
      DapObjectMongoDbRepository,
    );
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    const updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
    this.applyStateTransition = applyStateTransitionFactory(
      this.getIpfsApi(),
      updateDapContract,
      updateDapObject,
      this.options.getStorageIpfsTimeout(),
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
   * Create revertDapObjectsForStateTransition
   *
   * @returns {revertDapObjectsForStateTransition}
   */
  createRevertDapObjectsForStateTransition() {
    const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      this.getMongoClient(),
      DapObjectMongoDbRepository,
    );
    return revertDapObjectsForStateTransitionFactory(
      this.getIpfsApi(),
      this.getRpcClient(),
      createDapObjectMongoDbRepository,
      this.createApplyStateTransition(),
      this.createApplyStateTransitionFromReference(),
      this.options.getStorageIpfsTimeout(),
    );
  }

  /**
   * Create revertDapContractsForStateTransition
   *
   * @returns {revertDapContractsForStateTransition}
   */
  createRevertDapContractsForStateTransition() {
    const mongoDb = this.getMongoClient().db(this.options.getStorageMongoDbDatabase());
    const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    return revertDapContractsForStateTransitionFactory(
      dapContractMongoDbRepository,
      this.getRpcClient(),
      this.createApplyStateTransition(),
      this.createApplyStateTransitionFromReference(),
    );
  }
}

module.exports = SyncApp;

const IpfsAPI = require('ipfs-http-client');
const RpcClient = require('@dashevo/dashd-rpc/promise');
const { MongoClient } = require('mongodb');
const DashPlatformProtocol = require('@dashevo/dpp');

const sanitizer = require('../mongoDb/sanitizer');

const SyncStateRepository = require('../sync/state/repository/SyncStateRepository');
const SyncStateRepositoryChangeListener = require('../../lib/sync/state/repository/SyncStateRepositoryChangeListener');

const isSynced = require('../../lib/sync/isSynced');
const getCheckSyncStateHttpMiddleware = require('../../lib/sync/getCheckSyncHttpMiddleware');

const wrapToErrorHandler = require('../../lib/api/jsonRpc/wrapToErrorHandler');

const STPacketIpfsRepository = require('../storage/stPacket/STPacketIpfsRepository');
const addSTPacketFactory = require('../../lib/storage/stPacket/addSTPacketFactory');
const addSTPacketMethodFactory = require('../../lib/api/methods/addSTPacketMethodFactory');
const removeSTPacketFactory = require('../../lib/storage/stPacket/removeSTPacketFactory');
const removeSTPacketMethodFactory = require('../../lib/api/methods/removeSTPacketMethodFactory');

const SVContractMongoDbRepository = require('../stateView/contract/SVContractMongoDbRepository');
const createCIDFromHash = require('../storage/stPacket/createCIDFromHash');

const fetchContractFactory = require('../stateView/contract/fetchContractFactory');
const fetchContractMethodFactory = require('../api/methods/fetchContractMethodFactory');

const SVDocumentMongoDbRepository = require('../stateView/document/SVDocumentMongoDbRepository');
const createSVDocumentMongoDbRepositoryFactory = require('../stateView/document/createSVDocumentMongoDbRepositoryFactory');
const fetchDocumentsFactory = require('../stateView/document/fetchDocumentsFactory');
const fetchDocumentsMethodFactory = require('../api/methods/fetchDocumentsMethodFactory');

const getChainInfoFactory = require('../../lib/sync/info/chain/getChainInfoFactory');
const getSyncInfoFactory = require('../../lib/sync/info/getSyncInfoFactory');
const getSyncInfoMethodFactory = require('../../lib/api/methods/getSyncInfoMethodFactory');

const isDashCoreRunningFactory = require('../../lib/sync/isDashCoreRunningFactory');
const DashCoreIsNotRunningError = require('../../lib/sync/DashCoreIsNotRunningError');

const DriveDataProvider = require('../dpp/DriveDataProvider');

/**
 * Remove 'Method' Postfix
 *
 * Takes a function as an argument, returns the function's name
 * as a string without 'Method' as a postfix.
 *
 * @param {function} func Function that uses 'Method' postfix
 * @returns {string} String of function name without 'Method' postfix
 */
function rmPostfix(func) {
  const funcName = func.name;

  return funcName.substr(0, funcName.length - 'Method'.length);
}

class ApiApp {
  /**
   * @param {ApiAppOptions} options
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
  }

  /**
   * Init ApiApp
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
  }

  /**
   * Create check sync state http middleware
   *
   * @return {Function}
   */
  createCheckSyncStateHttpMiddleware() {
    const repositoryChangeListener = new SyncStateRepositoryChangeListener(
      this.syncStateRepository,
      this.options.getSyncStateCheckInterval(),
    );

    return getCheckSyncStateHttpMiddleware(
      isSynced,
      this.createGetSyncInfo(),
      repositoryChangeListener,
      this.options.getSyncChainCheckInterval(),
    );
  }

  /**
   * Create RPC methods with names
   *
   * @return {{string: Function}}
   */
  createRpcMethodsWithNames() {
    const methods = {};

    for (const method of this.createRpcMethods()) {
      methods[rmPostfix(method)] = wrapToErrorHandler(method);
    }

    return methods;
  }

  /**
   * @private
   * @return {[ Function ]}
   */
  createRpcMethods() {
    return [
      this.createAddSTPacketMethod(),
      this.createRemoveSTPacketMethod(),
      this.createFetchContractMethod(),
      this.createFetchDocumentsMethod(),
      this.createGetSyncInfoMethod(),
    ];
  }

  /**
   * @private
   * @return {STPacketIpfsRepository}
   */
  createSTPacketRepository() {
    if (this.stPacketRepository) {
      return this.stPacketRepository;
    }
    this.stPacketRepository = new STPacketIpfsRepository(
      this.ipfsAPI,
      this.createDashPlatformProtocol(),
      this.options.getStorageIpfsTimeout(),
    );
    return this.stPacketRepository;
  }

  /**
   * @private
   * @return {addSTPacketMethod}
   */
  createAddSTPacketMethod() {
    const addSTPacket = addSTPacketFactory(
      this.createSTPacketRepository(),
      this.createDashPlatformProtocol(),
    );

    return addSTPacketMethodFactory(
      addSTPacket,
      this.createDashPlatformProtocol(),
    );
  }

  /**
   * @private
   * @return {removeSTPacketMethod}
   */
  createRemoveSTPacketMethod() {
    const removeSTPacket = removeSTPacketFactory(this.createSTPacketRepository());
    return removeSTPacketMethodFactory(
      removeSTPacket,
      createCIDFromHash,
    );
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
        this.createDashPlatformProtocol(),
      );

      this.fetchContract = fetchContractFactory(svContractMongoDbRepository);
    }

    return this.fetchContract;
  }

  /**
   * @private
   * @returns {fetchContractMethod}
   */
  createFetchContractMethod() {
    return fetchContractMethodFactory(
      this.createFetchContract(),
    );
  }

  /**
   * @private
   * @return {fetchDocuments}
   */
  createFetchDocuments() {
    if (!this.fetchDocuments) {
      const createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
        this.mongoClient,
        SVDocumentMongoDbRepository,
        sanitizer,
      );
      this.fetchDocuments = fetchDocumentsFactory(createSVDocumentMongoDbRepository);
    }

    return this.fetchDocuments;
  }

  /**
   * @private
   * @return {fetchDocumentsMethod}
   */
  createFetchDocumentsMethod() {
    return fetchDocumentsMethodFactory(
      this.createFetchDocuments(),
    );
  }

  /**
   * @private
   * @return {getSyncInfo}
   */
  createGetSyncInfo() {
    if (!this.getSyncInfo) {
      const getChainInfo = getChainInfoFactory(this.rpcClient);
      this.getSyncInfo = getSyncInfoFactory(this.syncStateRepository, getChainInfo);
    }
    return this.getSyncInfo;
  }

  /**
   * @private
   * @return {getSyncInfoMethod}
   */
  createGetSyncInfoMethod() {
    return getSyncInfoMethodFactory(this.createGetSyncInfo());
  }

  /**
   * @private
   * @return {DashPlatformProtocol}
   */
  createDashPlatformProtocol() {
    if (!this.dashPlatfromProtocol) {
      const dataProvider = new DriveDataProvider(
        this.createFetchDocuments(),
        this.createFetchContract.bind(this),
        this.rpcClient,
      );

      this.dashPlatfromProtocol = new DashPlatformProtocol({
        dataProvider,
      });
    }

    return this.dashPlatfromProtocol;
  }
}

module.exports = ApiApp;

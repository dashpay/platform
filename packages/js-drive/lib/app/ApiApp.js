const IpfsAPI = require('ipfs-api');
const RpcClient = require('@dashevo/dashd-rpc/promise');
const { MongoClient } = require('mongodb');

const SyncStateRepository = require('../sync/state/repository/SyncStateRepository');
const SyncStateRepositoryChangeListener = require('../../lib/sync/state/repository/SyncStateRepositoryChangeListener');

const isSynced = require('../../lib/sync/isSynced');
const getCheckSyncStateHttpMiddleware = require('../../lib/sync/getCheckSyncHttpMiddleware');

const wrapToErrorHandler = require('../../lib/api/jsonRpc/wrapToErrorHandler');

const addSTPacketFactory = require('../../lib/storage/ipfs/addSTPacketFactory');
const addSTPacketMethodFactory = require('../../lib/api/methods/addSTPacketMethodFactory');

const DapContractMongoDbRepository = require('../stateView/dapContract/DapContractMongoDbRepository');
const serializer = require('../util/serializer');

const fetchDapContractFactory = require('../../lib/stateView/dapContract/fetchDapContractFactory');
const fetchDapContractMethodFactory = require('../api/methods/fetchDapContractMethodFactory');

const DapObjectMongoDbRepository = require('../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const fetchDapObjectsFactory = require('../../lib/stateView/dapObject/fetchDapObjectsFactory');
const fetchDapObjectsMethodFactory = require('../../lib/api/methods/fetchDapObjectsMethodFactory');

const getChainInfoFactory = require('../../lib/sync/info/chain/getChainInfoFactory');
const getSyncInfoFactory = require('../../lib/sync/info/getSyncInfoFactory');
const getSyncInfoMethodFactory = require('../../lib/api/methods/getSyncInfoMethodFactory');

const isDashCoreRunningFactory = require('../../lib/sync/isDashCoreRunningFactory');
const DashCoreIsNotRunningError = require('../../lib/sync/DashCoreIsNotRunningError');

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
      this.createFetchDapContractMethod(),
      this.createFetchDapObjectsMethod(),
      this.createGetSyncInfoMethod(),
    ];
  }


  /**
   * @private
   * @return {addSTPacketMethod}
   */
  createAddSTPacketMethod() {
    const addSTPacket = addSTPacketFactory(this.ipfsAPI);
    return addSTPacketMethodFactory(addSTPacket);
  }

  /**
   * @private
   * @returns {fetchDapContractMethod}
   */
  createFetchDapContractMethod() {
    const mongoDb = this.mongoClient.db(this.options.getStorageMongoDbDatabase());
    const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, serializer);
    const fetchDapContract = fetchDapContractFactory(dapContractMongoDbRepository);
    return fetchDapContractMethodFactory(fetchDapContract);
  }

  /**
   * @private
   * @return {fetchDapObjectsMethod}
   */
  createFetchDapObjectsMethod() {
    const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      this.mongoClient,
      DapObjectMongoDbRepository,
    );
    const fetchDapObjects = fetchDapObjectsFactory(createDapObjectMongoDbRepository);
    return fetchDapObjectsMethodFactory(fetchDapObjects);
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
}

module.exports = ApiApp;

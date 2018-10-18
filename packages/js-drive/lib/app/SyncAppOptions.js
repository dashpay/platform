class SyncAppOptions {
  constructor(options) {
    this.dashCoreJsonRpcHost = options.DASHCORE_JSON_RPC_HOST;
    this.dashCoreJsonRpcPort = options.DASHCORE_JSON_RPC_PORT;
    this.dashCoreJsonRpcUser = options.DASHCORE_JSON_RPC_USER;
    this.dashCoreJsonRpcPass = options.DASHCORE_JSON_RPC_PASS;
    this.dashCoreRunningCheckMaxRetries = parseInt(options.DASHCORE_RUNNING_CHECK_MAX_RETRIES, 10);
    this.dashCoreRunningCheckInterval = parseInt(options.DASHCORE_RUNNING_CHECK_INTERVAL, 10);
    this.dashCoreZmqPubHashblock = options.DASHCORE_ZMQ_PUB_HASHBLOCK;
    this.storageIpfsMultiAddr = options.STORAGE_IPFS_MULTIADDR;
    this.storageIpfsTimeout = parseInt(options.STORAGE_IPFS_TIMEOUT, 10);
    this.storageMongoDbUrl = options.STORAGE_MONGODB_URL;
    this.storageMongoDbDatabase = options.STORAGE_MONGODB_DB;
    this.syncEvoStartBlockHeight = parseInt(options.SYNC_EVO_START_BLOCK_HEIGHT, 10);
    this.syncStateBlocksLimit = options.SYNC_STATE_BLOCKS_LIMIT;
    this.mongoDbPrefix = options.MONGODB_DB_PREFIX;
  }

  getDashCoreJsonRpcHost() {
    return this.dashCoreJsonRpcHost;
  }

  getDashCoreJsonRpcPort() {
    return this.dashCoreJsonRpcPort;
  }

  getDashCoreJsonRpcUser() {
    return this.dashCoreJsonRpcUser;
  }

  getDashCoreJsonRpcPass() {
    return this.dashCoreJsonRpcPass;
  }

  getDashCoreRunningCheckMaxRetries() {
    return this.dashCoreRunningCheckMaxRetries;
  }

  getDashCoreRunningCheckInterval() {
    return this.dashCoreRunningCheckInterval;
  }

  getDashCoreZmqPubHashBlock() {
    return this.dashCoreZmqPubHashblock;
  }

  getStorageIpfsMultiAddr() {
    return this.storageIpfsMultiAddr;
  }

  /**
   * @return {number}
   */
  getStorageIpfsTimeout() {
    return this.storageIpfsTimeout;
  }

  getStorageMongoDbUrl() {
    return this.storageMongoDbUrl;
  }

  getStorageMongoDbDatabase() {
    return this.storageMongoDbDatabase;
  }

  getSyncEvoStartBlockHeight() {
    return this.syncEvoStartBlockHeight;
  }

  getSyncStateBlocksLimit() {
    return this.syncStateBlocksLimit;
  }

  getMongoDbPrefix() {
    return this.mongoDbPrefix;
  }
}

module.exports = SyncAppOptions;

class ApiAppOptions {
  constructor(options) {
    this.storageMongoDbUrl = options.STORAGE_MONGODB_URL;
    this.storageMongoDbDatabase = options.STORAGE_MONGODB_DB;
    this.apiRpcPort = options.API_RPC_PORT;
    this.apiRpcHost = options.API_RPC_HOST;
  }

  getStorageMongoDbUrl() {
    return this.storageMongoDbUrl;
  }

  getStorageMongoDbDatabase() {
    return this.storageMongoDbDatabase;
  }

  getApiRpcHost() {
    return this.apiRpcHost;
  }

  getApiRpcPort() {
    return this.apiRpcPort;
  }
}

module.exports = ApiAppOptions;

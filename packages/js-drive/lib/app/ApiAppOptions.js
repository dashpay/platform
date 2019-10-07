class ApiAppOptions {
  constructor(options) {
    this.stateViewMongoDBUrl = options.STATEVIEW_MONGODB_URL;
    this.stateViewMongoDBDatabase = options.STATEVIEW_MONGODB_DB;
    this.apiRpcPort = options.API_RPC_PORT;
    this.apiRpcHost = options.API_RPC_HOST;
  }

  getStateViewMongoDBUrl() {
    return this.stateViewMongoDBUrl;
  }

  getStateViewMongoDBDatabase() {
    return this.stateViewMongoDBDatabase;
  }

  getApiRpcHost() {
    return this.apiRpcHost;
  }

  getApiRpcPort() {
    return this.apiRpcPort;
  }
}

module.exports = ApiAppOptions;

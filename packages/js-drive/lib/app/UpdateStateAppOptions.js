class UpdateStateAppOptions {
  constructor(options) {
    this.storageMongoDbUrl = options.STORAGE_MONGODB_URL;
    this.storageMongoDbDatabase = options.STORAGE_MONGODB_DB;
    this.gRpcHost = options.UPDATE_STATE_API_GRPC_HOST;
    this.gRpcPort = options.UPDATE_STATE_API_GRPC_PORT;
    this.dashCoreJsonRpcHost = options.DASHCORE_JSON_RPC_HOST;
    this.dashCoreJsonRpcPort = options.DASHCORE_JSON_RPC_PORT;
    this.dashCoreJsonRpcUser = options.DASHCORE_JSON_RPC_USER;
    this.dashCoreJsonRpcPass = options.DASHCORE_JSON_RPC_PASS;
  }

  getStorageMongoDbUrl() {
    return this.storageMongoDbUrl;
  }

  getStorageMongoDbDatabase() {
    return this.storageMongoDbDatabase;
  }

  getGrpcHost() {
    return this.gRpcHost;
  }

  getGrpcPort() {
    return this.gRpcPort;
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
}

module.exports = UpdateStateAppOptions;

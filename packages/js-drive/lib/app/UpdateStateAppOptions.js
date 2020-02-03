class UpdateStateAppOptions {
  constructor(options) {
    this.stateViewMongoDBUrl = options.STATEVIEW_MONGODB_URL;
    this.stateViewMongoDBDatabase = options.STATEVIEW_MONGODB_DB;
    this.gRpcHost = options.UPDATE_STATE_GRPC_HOST;
    this.gRpcPort = options.UPDATE_STATE_GRPC_PORT;
    this.dashCoreJsonRpcHost = options.DASHCORE_JSON_RPC_HOST;
    this.dashCoreJsonRpcPort = options.DASHCORE_JSON_RPC_PORT;
    this.dashCoreJsonRpcUser = options.DASHCORE_JSON_RPC_USER;
    this.dashCoreJsonRpcPass = options.DASHCORE_JSON_RPC_PASS;
    this.tendermintRpcHost = options.TENDERMINT_RPC_HOST;
    this.tendermintRpcPort = options.TENDERMINT_RPC_PORT;
  }

  getStateViewMongoDBUrl() {
    return this.stateViewMongoDBUrl;
  }

  getStateViewMongoDBDatabase() {
    return this.stateViewMongoDBDatabase;
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

  getTendermintRpcHost() {
    return this.tendermintRpcHost;
  }

  getTendermintRpcPort() {
    return this.tendermintRpcPort;
  }
}

module.exports = UpdateStateAppOptions;

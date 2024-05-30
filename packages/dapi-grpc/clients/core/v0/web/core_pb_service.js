// package: org.dash.platform.dapi.v0
// file: core.proto

var core_pb = require("./core_pb");
var grpc = require("@improbable-eng/grpc-web").grpc;

var Core = (function () {
  function Core() {}
  Core.serviceName = "org.dash.platform.dapi.v0.Core";
  return Core;
}());

Core.getBlockchainStatus = {
  methodName: "getBlockchainStatus",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetBlockchainStatusRequest,
  responseType: core_pb.GetBlockchainStatusResponse
};

Core.getMasternodeStatus = {
  methodName: "getMasternodeStatus",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetMasternodeStatusRequest,
  responseType: core_pb.GetMasternodeStatusResponse
};

Core.getBlock = {
  methodName: "getBlock",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetBlockRequest,
  responseType: core_pb.GetBlockResponse
};

Core.getBestBlockHeight = {
  methodName: "getBestBlockHeight",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetBestBlockHeightRequest,
  responseType: core_pb.GetBestBlockHeightResponse
};

Core.broadcastTransaction = {
  methodName: "broadcastTransaction",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.BroadcastTransactionRequest,
  responseType: core_pb.BroadcastTransactionResponse
};

Core.getTransaction = {
  methodName: "getTransaction",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetTransactionRequest,
  responseType: core_pb.GetTransactionResponse
};

Core.getEstimatedTransactionFee = {
  methodName: "getEstimatedTransactionFee",
  service: Core,
  requestStream: false,
  responseStream: false,
  requestType: core_pb.GetEstimatedTransactionFeeRequest,
  responseType: core_pb.GetEstimatedTransactionFeeResponse
};

Core.subscribeToBlockHeadersWithChainLocks = {
  methodName: "subscribeToBlockHeadersWithChainLocks",
  service: Core,
  requestStream: false,
  responseStream: true,
  requestType: core_pb.BlockHeadersWithChainLocksRequest,
  responseType: core_pb.BlockHeadersWithChainLocksResponse
};

Core.subscribeToTransactionsWithProofs = {
  methodName: "subscribeToTransactionsWithProofs",
  service: Core,
  requestStream: false,
  responseStream: true,
  requestType: core_pb.TransactionsWithProofsRequest,
  responseType: core_pb.TransactionsWithProofsResponse
};

Core.subscribeToMasternodeList = {
  methodName: "subscribeToMasternodeList",
  service: Core,
  requestStream: false,
  responseStream: true,
  requestType: core_pb.MasternodeListRequest,
  responseType: core_pb.MasternodeListResponse
};

exports.Core = Core;

function CoreClient(serviceHost, options) {
  this.serviceHost = serviceHost;
  this.options = options || {};
}

CoreClient.prototype.getBlockchainStatus = function getBlockchainStatus(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getBlockchainStatus, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.getMasternodeStatus = function getMasternodeStatus(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getMasternodeStatus, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.getBlock = function getBlock(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getBlock, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.getBestBlockHeight = function getBestBlockHeight(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getBestBlockHeight, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.broadcastTransaction = function broadcastTransaction(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.broadcastTransaction, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.getTransaction = function getTransaction(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getTransaction, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.getEstimatedTransactionFee = function getEstimatedTransactionFee(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Core.getEstimatedTransactionFee, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onEnd: function (response) {
      if (callback) {
        if (response.status !== grpc.Code.OK) {
          var err = new Error(response.statusMessage);
          err.code = response.status;
          err.metadata = response.trailers;
          callback(err, null);
        } else {
          callback(null, response.message);
        }
      }
    }
  });
  return {
    cancel: function () {
      callback = null;
      client.close();
    }
  };
};

CoreClient.prototype.subscribeToBlockHeadersWithChainLocks = function subscribeToBlockHeadersWithChainLocks(requestMessage, metadata) {
  var listeners = {
    data: [],
    end: [],
    status: []
  };
  var client = grpc.invoke(Core.subscribeToBlockHeadersWithChainLocks, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onMessage: function (responseMessage) {
      listeners.data.forEach(function (handler) {
        handler(responseMessage);
      });
    },
    onEnd: function (status, statusMessage, trailers) {
      listeners.status.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners.end.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners = null;
    }
  });
  return {
    on: function (type, handler) {
      listeners[type].push(handler);
      return this;
    },
    cancel: function () {
      listeners = null;
      client.close();
    }
  };
};

CoreClient.prototype.subscribeToTransactionsWithProofs = function subscribeToTransactionsWithProofs(requestMessage, metadata) {
  var listeners = {
    data: [],
    end: [],
    status: []
  };
  var client = grpc.invoke(Core.subscribeToTransactionsWithProofs, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onMessage: function (responseMessage) {
      listeners.data.forEach(function (handler) {
        handler(responseMessage);
      });
    },
    onEnd: function (status, statusMessage, trailers) {
      listeners.status.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners.end.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners = null;
    }
  });
  return {
    on: function (type, handler) {
      listeners[type].push(handler);
      return this;
    },
    cancel: function () {
      listeners = null;
      client.close();
    }
  };
};

CoreClient.prototype.subscribeToMasternodeList = function subscribeToMasternodeList(requestMessage, metadata) {
  var listeners = {
    data: [],
    end: [],
    status: []
  };
  var client = grpc.invoke(Core.subscribeToMasternodeList, {
    request: requestMessage,
    host: this.serviceHost,
    metadata: metadata,
    transport: this.options.transport,
    debug: this.options.debug,
    onMessage: function (responseMessage) {
      listeners.data.forEach(function (handler) {
        handler(responseMessage);
      });
    },
    onEnd: function (status, statusMessage, trailers) {
      listeners.status.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners.end.forEach(function (handler) {
        handler({ code: status, details: statusMessage, metadata: trailers });
      });
      listeners = null;
    }
  });
  return {
    on: function (type, handler) {
      listeners[type].push(handler);
      return this;
    },
    cancel: function () {
      listeners = null;
      client.close();
    }
  };
};

exports.CoreClient = CoreClient;


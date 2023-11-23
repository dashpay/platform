// package: org.dash.platform.dapi.v0
// file: platform.proto

var platform_pb = require("./platform_pb");
var grpc = require("@improbable-eng/grpc-web").grpc;

var Platform = (function () {
  function Platform() {}
  Platform.serviceName = "org.dash.platform.dapi.v0.Platform";
  return Platform;
}());

Platform.broadcastStateTransition = {
  methodName: "broadcastStateTransition",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.BroadcastStateTransitionRequest,
  responseType: platform_pb.BroadcastStateTransitionResponse
};

Platform.getIdentity = {
  methodName: "getIdentity",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityRequest,
  responseType: platform_pb.GetIdentityResponse
};

Platform.getIdentities = {
  methodName: "getIdentities",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentitiesRequest,
  responseType: platform_pb.GetIdentitiesResponse
};

Platform.getIdentityKeys = {
  methodName: "getIdentityKeys",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityKeysRequest,
  responseType: platform_pb.GetIdentityKeysResponse
};

Platform.getIdentityBalance = {
  methodName: "getIdentityBalance",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityBalanceRequest,
  responseType: platform_pb.GetIdentityBalanceResponse
};

Platform.getIdentityBalanceAndRevision = {
  methodName: "getIdentityBalanceAndRevision",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityBalanceAndRevisionRequest,
  responseType: platform_pb.GetIdentityBalanceAndRevisionResponse
};

Platform.getProofs = {
  methodName: "getProofs",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetProofsRequest,
  responseType: platform_pb.GetProofsResponse
};

Platform.getDataContract = {
  methodName: "getDataContract",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetDataContractRequest,
  responseType: platform_pb.GetDataContractResponse
};

Platform.getDataContractHistory = {
  methodName: "getDataContractHistory",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetDataContractHistoryRequest,
  responseType: platform_pb.GetDataContractHistoryResponse
};

Platform.getDataContracts = {
  methodName: "getDataContracts",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetDataContractsRequest,
  responseType: platform_pb.GetDataContractsResponse
};

Platform.getDocuments = {
  methodName: "getDocuments",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetDocumentsRequest,
  responseType: platform_pb.GetDocumentsResponse
};

Platform.getIdentitiesByPublicKeyHashes = {
  methodName: "getIdentitiesByPublicKeyHashes",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentitiesByPublicKeyHashesRequest,
  responseType: platform_pb.GetIdentitiesByPublicKeyHashesResponse
};

Platform.getIdentityByPublicKeyHash = {
  methodName: "getIdentityByPublicKeyHash",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityByPublicKeyHashRequest,
  responseType: platform_pb.GetIdentityByPublicKeyHashResponse
};

Platform.waitForStateTransitionResult = {
  methodName: "waitForStateTransitionResult",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.WaitForStateTransitionResultRequest,
  responseType: platform_pb.WaitForStateTransitionResultResponse
};

Platform.getConsensusParams = {
  methodName: "getConsensusParams",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetConsensusParamsRequest,
  responseType: platform_pb.GetConsensusParamsResponse
};

Platform.getProtocolVersionUpgradeState = {
  methodName: "getProtocolVersionUpgradeState",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetProtocolVersionUpgradeStateRequest,
  responseType: platform_pb.GetProtocolVersionUpgradeStateResponse
};

Platform.getProtocolVersionUpgradeVoteStatus = {
  methodName: "getProtocolVersionUpgradeVoteStatus",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetProtocolVersionUpgradeVoteStatusRequest,
  responseType: platform_pb.GetProtocolVersionUpgradeVoteStatusResponse
};

Platform.getEpochsInfo = {
  methodName: "getEpochsInfo",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetEpochsInfoRequest,
  responseType: platform_pb.GetEpochsInfoResponse
};

exports.Platform = Platform;

function PlatformClient(serviceHost, options) {
  this.serviceHost = serviceHost;
  this.options = options || {};
}

PlatformClient.prototype.broadcastStateTransition = function broadcastStateTransition(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.broadcastStateTransition, {
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

PlatformClient.prototype.getIdentity = function getIdentity(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentity, {
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

PlatformClient.prototype.getIdentities = function getIdentities(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentities, {
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

PlatformClient.prototype.getIdentityKeys = function getIdentityKeys(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityKeys, {
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

PlatformClient.prototype.getIdentityBalance = function getIdentityBalance(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityBalance, {
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

PlatformClient.prototype.getIdentityBalanceAndRevision = function getIdentityBalanceAndRevision(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityBalanceAndRevision, {
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

PlatformClient.prototype.getProofs = function getProofs(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getProofs, {
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

PlatformClient.prototype.getDataContract = function getDataContract(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getDataContract, {
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

PlatformClient.prototype.getDataContractHistory = function getDataContractHistory(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getDataContractHistory, {
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

PlatformClient.prototype.getDataContracts = function getDataContracts(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getDataContracts, {
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

PlatformClient.prototype.getDocuments = function getDocuments(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getDocuments, {
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

PlatformClient.prototype.getIdentitiesByPublicKeyHashes = function getIdentitiesByPublicKeyHashes(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentitiesByPublicKeyHashes, {
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

PlatformClient.prototype.getIdentityByPublicKeyHash = function getIdentityByPublicKeyHash(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityByPublicKeyHash, {
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

PlatformClient.prototype.waitForStateTransitionResult = function waitForStateTransitionResult(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.waitForStateTransitionResult, {
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

PlatformClient.prototype.getConsensusParams = function getConsensusParams(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getConsensusParams, {
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

PlatformClient.prototype.getProtocolVersionUpgradeState = function getProtocolVersionUpgradeState(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getProtocolVersionUpgradeState, {
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

PlatformClient.prototype.getProtocolVersionUpgradeVoteStatus = function getProtocolVersionUpgradeVoteStatus(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getProtocolVersionUpgradeVoteStatus, {
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

PlatformClient.prototype.getEpochsInfo = function getEpochsInfo(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getEpochsInfo, {
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

exports.PlatformClient = PlatformClient;


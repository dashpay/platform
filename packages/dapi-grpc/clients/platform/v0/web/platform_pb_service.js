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

Platform.getIdentityKeys = {
  methodName: "getIdentityKeys",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityKeysRequest,
  responseType: platform_pb.GetIdentityKeysResponse
};

Platform.getIdentitiesContractKeys = {
  methodName: "getIdentitiesContractKeys",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentitiesContractKeysRequest,
  responseType: platform_pb.GetIdentitiesContractKeysResponse
};

Platform.getIdentityNonce = {
  methodName: "getIdentityNonce",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityNonceRequest,
  responseType: platform_pb.GetIdentityNonceResponse
};

Platform.getIdentityContractNonce = {
  methodName: "getIdentityContractNonce",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityContractNonceRequest,
  responseType: platform_pb.GetIdentityContractNonceResponse
};

Platform.getIdentityBalance = {
  methodName: "getIdentityBalance",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityBalanceRequest,
  responseType: platform_pb.GetIdentityBalanceResponse
};

Platform.getIdentitiesBalances = {
  methodName: "getIdentitiesBalances",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentitiesBalancesRequest,
  responseType: platform_pb.GetIdentitiesBalancesResponse
};

Platform.getIdentityBalanceAndRevision = {
  methodName: "getIdentityBalanceAndRevision",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetIdentityBalanceAndRevisionRequest,
  responseType: platform_pb.GetIdentityBalanceAndRevisionResponse
};

Platform.getEvonodesProposedEpochBlocksByIds = {
  methodName: "getEvonodesProposedEpochBlocksByIds",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetEvonodesProposedEpochBlocksByIdsRequest,
  responseType: platform_pb.GetEvonodesProposedEpochBlocksResponse
};

Platform.getEvonodesProposedEpochBlocksByRange = {
  methodName: "getEvonodesProposedEpochBlocksByRange",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetEvonodesProposedEpochBlocksByRangeRequest,
  responseType: platform_pb.GetEvonodesProposedEpochBlocksResponse
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

Platform.getContestedResources = {
  methodName: "getContestedResources",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetContestedResourcesRequest,
  responseType: platform_pb.GetContestedResourcesResponse
};

Platform.getContestedResourceVoteState = {
  methodName: "getContestedResourceVoteState",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetContestedResourceVoteStateRequest,
  responseType: platform_pb.GetContestedResourceVoteStateResponse
};

Platform.getContestedResourceVotersForIdentity = {
  methodName: "getContestedResourceVotersForIdentity",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetContestedResourceVotersForIdentityRequest,
  responseType: platform_pb.GetContestedResourceVotersForIdentityResponse
};

Platform.getContestedResourceIdentityVotes = {
  methodName: "getContestedResourceIdentityVotes",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetContestedResourceIdentityVotesRequest,
  responseType: platform_pb.GetContestedResourceIdentityVotesResponse
};

Platform.getVotePollsByEndDate = {
  methodName: "getVotePollsByEndDate",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetVotePollsByEndDateRequest,
  responseType: platform_pb.GetVotePollsByEndDateResponse
};

Platform.getPrefundedSpecializedBalance = {
  methodName: "getPrefundedSpecializedBalance",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetPrefundedSpecializedBalanceRequest,
  responseType: platform_pb.GetPrefundedSpecializedBalanceResponse
};

Platform.getTotalCreditsInPlatform = {
  methodName: "getTotalCreditsInPlatform",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetTotalCreditsInPlatformRequest,
  responseType: platform_pb.GetTotalCreditsInPlatformResponse
};

Platform.getPathElements = {
  methodName: "getPathElements",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetPathElementsRequest,
  responseType: platform_pb.GetPathElementsResponse
};

Platform.getStatus = {
  methodName: "getStatus",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetStatusRequest,
  responseType: platform_pb.GetStatusResponse
};

Platform.getCurrentQuorumsInfo = {
  methodName: "getCurrentQuorumsInfo",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetCurrentQuorumsInfoRequest,
  responseType: platform_pb.GetCurrentQuorumsInfoResponse
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

PlatformClient.prototype.getIdentitiesContractKeys = function getIdentitiesContractKeys(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentitiesContractKeys, {
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

PlatformClient.prototype.getIdentityNonce = function getIdentityNonce(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityNonce, {
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

PlatformClient.prototype.getIdentityContractNonce = function getIdentityContractNonce(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentityContractNonce, {
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

PlatformClient.prototype.getIdentitiesBalances = function getIdentitiesBalances(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getIdentitiesBalances, {
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

PlatformClient.prototype.getEvonodesProposedEpochBlocksByIds = function getEvonodesProposedEpochBlocksByIds(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getEvonodesProposedEpochBlocksByIds, {
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

PlatformClient.prototype.getEvonodesProposedEpochBlocksByRange = function getEvonodesProposedEpochBlocksByRange(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getEvonodesProposedEpochBlocksByRange, {
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

PlatformClient.prototype.getContestedResources = function getContestedResources(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getContestedResources, {
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

PlatformClient.prototype.getContestedResourceVoteState = function getContestedResourceVoteState(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getContestedResourceVoteState, {
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

PlatformClient.prototype.getContestedResourceVotersForIdentity = function getContestedResourceVotersForIdentity(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getContestedResourceVotersForIdentity, {
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

PlatformClient.prototype.getContestedResourceIdentityVotes = function getContestedResourceIdentityVotes(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getContestedResourceIdentityVotes, {
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

PlatformClient.prototype.getVotePollsByEndDate = function getVotePollsByEndDate(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getVotePollsByEndDate, {
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

PlatformClient.prototype.getPrefundedSpecializedBalance = function getPrefundedSpecializedBalance(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getPrefundedSpecializedBalance, {
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

PlatformClient.prototype.getTotalCreditsInPlatform = function getTotalCreditsInPlatform(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getTotalCreditsInPlatform, {
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

PlatformClient.prototype.getPathElements = function getPathElements(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getPathElements, {
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

PlatformClient.prototype.getStatus = function getStatus(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getStatus, {
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

PlatformClient.prototype.getCurrentQuorumsInfo = function getCurrentQuorumsInfo(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(Platform.getCurrentQuorumsInfo, {
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


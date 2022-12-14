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

Platform.getDataContract = {
  methodName: "getDataContract",
  service: Platform,
  requestStream: false,
  responseStream: false,
  requestType: platform_pb.GetDataContractRequest,
  responseType: platform_pb.GetDataContractResponse
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

exports.PlatformClient = PlatformClient;


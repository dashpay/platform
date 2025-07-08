// package: org.dash.platform.drive.v0
// file: drive.proto

var drive_pb = require("./drive_pb");
var grpc = require("@improbable-eng/grpc-web").grpc;

var DriveInternal = (function () {
  function DriveInternal() {}
  DriveInternal.serviceName = "org.dash.platform.drive.v0.DriveInternal";
  return DriveInternal;
}());

DriveInternal.getProofs = {
  methodName: "getProofs",
  service: DriveInternal,
  requestStream: false,
  responseStream: false,
  requestType: drive_pb.GetProofsRequest,
  responseType: drive_pb.GetProofsResponse
};

exports.DriveInternal = DriveInternal;

function DriveInternalClient(serviceHost, options) {
  this.serviceHost = serviceHost;
  this.options = options || {};
}

DriveInternalClient.prototype.getProofs = function getProofs(requestMessage, metadata, callback) {
  if (arguments.length === 2) {
    callback = arguments[1];
  }
  var client = grpc.unary(DriveInternal.getProofs, {
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

exports.DriveInternalClient = DriveInternalClient;


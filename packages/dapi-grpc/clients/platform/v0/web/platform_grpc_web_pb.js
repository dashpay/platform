/**
 * @fileoverview gRPC-Web generated client stub for org.dash.platform.dapi.v0
 * @enhanceable
 * @public
 */

// GENERATED CODE -- DO NOT EDIT!


/* eslint-disable */
// @ts-nocheck



const grpc = {};
grpc.web = require('grpc-web');

const proto = {};
proto.org = {};
proto.org.dash = {};
proto.org.dash.platform = {};
proto.org.dash.platform.dapi = {};
proto.org.dash.platform.dapi.v0 = require('./platform_pb.js');

/**
 * @param {string} hostname
 * @param {?Object} credentials
 * @param {?Object} options
 * @constructor
 * @struct
 * @final
 */
proto.org.dash.platform.dapi.v0.PlatformClient =
    function(hostname, credentials, options) {
  if (!options) options = {};
  options['format'] = 'text';

  /**
   * @private @const {!grpc.web.GrpcWebClientBase} The client
   */
  this.client_ = new grpc.web.GrpcWebClientBase(options);

  /**
   * @private @const {string} The hostname
   */
  this.hostname_ = hostname;

};


/**
 * @param {string} hostname
 * @param {?Object} credentials
 * @param {?Object} options
 * @constructor
 * @struct
 * @final
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient =
    function(hostname, credentials, options) {
  if (!options) options = {};
  options['format'] = 'text';

  /**
   * @private @const {!grpc.web.GrpcWebClientBase} The client
   */
  this.client_ = new grpc.web.GrpcWebClientBase(options);

  /**
   * @private @const {string} The hostname
   */
  this.hostname_ = hostname;

};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest,
 *   !proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>}
 */
const methodDescriptor_Platform_broadcastStateTransition = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/broadcastStateTransition',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest,
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest,
 *   !proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>}
 */
const methodInfo_Platform_broadcastStateTransition = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.broadcastStateTransition =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/broadcastStateTransition',
      request,
      metadata || {},
      methodDescriptor_Platform_broadcastStateTransition,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.broadcastStateTransition =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/broadcastStateTransition',
      request,
      metadata || {},
      methodDescriptor_Platform_broadcastStateTransition);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetIdentityRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentityResponse>}
 */
const methodDescriptor_Platform_getIdentity = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getIdentity',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetIdentityRequest,
  proto.org.dash.platform.dapi.v0.GetIdentityResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetIdentityRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentityResponse>}
 */
const methodInfo_Platform_getIdentity = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetIdentityResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetIdentityResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetIdentityResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getIdentity =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentity',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentity,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetIdentityResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getIdentity =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentity',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentity);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetDataContractRequest,
 *   !proto.org.dash.platform.dapi.v0.GetDataContractResponse>}
 */
const methodDescriptor_Platform_getDataContract = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getDataContract',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetDataContractRequest,
  proto.org.dash.platform.dapi.v0.GetDataContractResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetDataContractRequest,
 *   !proto.org.dash.platform.dapi.v0.GetDataContractResponse>}
 */
const methodInfo_Platform_getDataContract = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetDataContractResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetDataContractResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetDataContractResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getDataContract =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getDataContract',
      request,
      metadata || {},
      methodDescriptor_Platform_getDataContract,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetDataContractResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getDataContract =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getDataContract',
      request,
      metadata || {},
      methodDescriptor_Platform_getDataContract);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetDocumentsRequest,
 *   !proto.org.dash.platform.dapi.v0.GetDocumentsResponse>}
 */
const methodDescriptor_Platform_getDocuments = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getDocuments',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetDocumentsRequest,
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetDocumentsRequest,
 *   !proto.org.dash.platform.dapi.v0.GetDocumentsResponse>}
 */
const methodInfo_Platform_getDocuments = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetDocumentsResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetDocumentsResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getDocuments =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getDocuments',
      request,
      metadata || {},
      methodDescriptor_Platform_getDocuments,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetDocumentsResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getDocuments =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getDocuments',
      request,
      metadata || {},
      methodDescriptor_Platform_getDocuments);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse>}
 */
const methodDescriptor_Platform_getIdentitiesByPublicKeyHashes = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest,
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse>}
 */
const methodInfo_Platform_getIdentitiesByPublicKeyHashes = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getIdentitiesByPublicKeyHashes =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentitiesByPublicKeyHashes,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getIdentitiesByPublicKeyHashes =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentitiesByPublicKeyHashes);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse>}
 */
const methodDescriptor_Platform_getIdentityIdsByPublicKeyHashes = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getIdentityIdsByPublicKeyHashes',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest,
  proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest,
 *   !proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse>}
 */
const methodInfo_Platform_getIdentityIdsByPublicKeyHashes = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getIdentityIdsByPublicKeyHashes =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentityIdsByPublicKeyHashes',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentityIdsByPublicKeyHashes,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getIdentityIdsByPublicKeyHashes =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getIdentityIdsByPublicKeyHashes',
      request,
      metadata || {},
      methodDescriptor_Platform_getIdentityIdsByPublicKeyHashes);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest,
 *   !proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>}
 */
const methodDescriptor_Platform_waitForStateTransitionResult = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest,
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest,
 *   !proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>}
 */
const methodInfo_Platform_waitForStateTransitionResult = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.waitForStateTransitionResult =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult',
      request,
      metadata || {},
      methodDescriptor_Platform_waitForStateTransitionResult,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.waitForStateTransitionResult =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult',
      request,
      metadata || {},
      methodDescriptor_Platform_waitForStateTransitionResult);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest,
 *   !proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse>}
 */
const methodDescriptor_Platform_getConsensusParams = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Platform/getConsensusParams',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest,
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest,
 *   !proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse>}
 */
const methodInfo_Platform_getConsensusParams = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.PlatformClient.prototype.getConsensusParams =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getConsensusParams',
      request,
      metadata || {},
      methodDescriptor_Platform_getConsensusParams,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.PlatformPromiseClient.prototype.getConsensusParams =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Platform/getConsensusParams',
      request,
      metadata || {},
      methodDescriptor_Platform_getConsensusParams);
};


module.exports = proto.org.dash.platform.dapi.v0;


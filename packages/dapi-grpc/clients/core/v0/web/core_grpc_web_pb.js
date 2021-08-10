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
proto.org.dash.platform.dapi.v0 = require('./core_pb.js');

/**
 * @param {string} hostname
 * @param {?Object} credentials
 * @param {?Object} options
 * @constructor
 * @struct
 * @final
 */
proto.org.dash.platform.dapi.v0.CoreClient =
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
proto.org.dash.platform.dapi.v0.CorePromiseClient =
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
 *   !proto.org.dash.platform.dapi.v0.GetStatusRequest,
 *   !proto.org.dash.platform.dapi.v0.GetStatusResponse>}
 */
const methodDescriptor_Core_getStatus = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/getStatus',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetStatusRequest,
  proto.org.dash.platform.dapi.v0.GetStatusResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetStatusRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetStatusResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetStatusRequest,
 *   !proto.org.dash.platform.dapi.v0.GetStatusResponse>}
 */
const methodInfo_Core_getStatus = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetStatusResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetStatusRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetStatusResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetStatusRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetStatusResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetStatusResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.getStatus =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getStatus',
      request,
      metadata || {},
      methodDescriptor_Core_getStatus,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetStatusRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetStatusResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.getStatus =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getStatus',
      request,
      metadata || {},
      methodDescriptor_Core_getStatus);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetBlockRequest,
 *   !proto.org.dash.platform.dapi.v0.GetBlockResponse>}
 */
const methodDescriptor_Core_getBlock = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/getBlock',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetBlockRequest,
  proto.org.dash.platform.dapi.v0.GetBlockResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetBlockRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetBlockResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetBlockRequest,
 *   !proto.org.dash.platform.dapi.v0.GetBlockResponse>}
 */
const methodInfo_Core_getBlock = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetBlockResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetBlockRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetBlockResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetBlockRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetBlockResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetBlockResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.getBlock =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getBlock',
      request,
      metadata || {},
      methodDescriptor_Core_getBlock,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetBlockRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetBlockResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.getBlock =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getBlock',
      request,
      metadata || {},
      methodDescriptor_Core_getBlock);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest,
 *   !proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse>}
 */
const methodDescriptor_Core_broadcastTransaction = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/broadcastTransaction',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest,
  proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest,
 *   !proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse>}
 */
const methodInfo_Core_broadcastTransaction = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.broadcastTransaction =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/broadcastTransaction',
      request,
      metadata || {},
      methodDescriptor_Core_broadcastTransaction,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastTransactionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.BroadcastTransactionResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.broadcastTransaction =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/broadcastTransaction',
      request,
      metadata || {},
      methodDescriptor_Core_broadcastTransaction);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetTransactionRequest,
 *   !proto.org.dash.platform.dapi.v0.GetTransactionResponse>}
 */
const methodDescriptor_Core_getTransaction = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/getTransaction',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetTransactionRequest,
  proto.org.dash.platform.dapi.v0.GetTransactionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetTransactionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetTransactionResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetTransactionRequest,
 *   !proto.org.dash.platform.dapi.v0.GetTransactionResponse>}
 */
const methodInfo_Core_getTransaction = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetTransactionResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetTransactionRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetTransactionResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetTransactionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetTransactionResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetTransactionResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.getTransaction =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getTransaction',
      request,
      metadata || {},
      methodDescriptor_Core_getTransaction,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetTransactionRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetTransactionResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.getTransaction =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getTransaction',
      request,
      metadata || {},
      methodDescriptor_Core_getTransaction);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest,
 *   !proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse>}
 */
const methodDescriptor_Core_getEstimatedTransactionFee = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee',
  grpc.web.MethodType.UNARY,
  proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest,
  proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest,
 *   !proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse>}
 */
const methodInfo_Core_getEstimatedTransactionFee = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @param {function(?grpc.web.Error, ?proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse)}
 *     callback The callback function(error, response)
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse>|undefined}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.getEstimatedTransactionFee =
    function(request, metadata, callback) {
  return this.client_.rpcCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee',
      request,
      metadata || {},
      methodDescriptor_Core_getEstimatedTransactionFee,
      callback);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} request The
 *     request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!Promise<!proto.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse>}
 *     Promise that resolves to the response
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.getEstimatedTransactionFee =
    function(request, metadata) {
  return this.client_.unaryCall(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee',
      request,
      metadata || {},
      methodDescriptor_Core_getEstimatedTransactionFee);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest,
 *   !proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse>}
 */
const methodDescriptor_Core_subscribeToBlockHeadersWithChainLocks = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks',
  grpc.web.MethodType.SERVER_STREAMING,
  proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest,
  proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest,
 *   !proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse>}
 */
const methodInfo_Core_subscribeToBlockHeadersWithChainLocks = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} request The request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse>}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.subscribeToBlockHeadersWithChainLocks =
    function(request, metadata) {
  return this.client_.serverStreaming(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks',
      request,
      metadata || {},
      methodDescriptor_Core_subscribeToBlockHeadersWithChainLocks);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} request The request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse>}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.subscribeToBlockHeadersWithChainLocks =
    function(request, metadata) {
  return this.client_.serverStreaming(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks',
      request,
      metadata || {},
      methodDescriptor_Core_subscribeToBlockHeadersWithChainLocks);
};


/**
 * @const
 * @type {!grpc.web.MethodDescriptor<
 *   !proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest,
 *   !proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse>}
 */
const methodDescriptor_Core_subscribeToTransactionsWithProofs = new grpc.web.MethodDescriptor(
  '/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs',
  grpc.web.MethodType.SERVER_STREAMING,
  proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest,
  proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse.deserializeBinary
);


/**
 * @const
 * @type {!grpc.web.AbstractClientBase.MethodInfo<
 *   !proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest,
 *   !proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse>}
 */
const methodInfo_Core_subscribeToTransactionsWithProofs = new grpc.web.AbstractClientBase.MethodInfo(
  proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse,
  /**
   * @param {!proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest} request
   * @return {!Uint8Array}
   */
  function(request) {
    return request.serializeBinary();
  },
  proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse.deserializeBinary
);


/**
 * @param {!proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest} request The request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse>}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CoreClient.prototype.subscribeToTransactionsWithProofs =
    function(request, metadata) {
  return this.client_.serverStreaming(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs',
      request,
      metadata || {},
      methodDescriptor_Core_subscribeToTransactionsWithProofs);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.TransactionsWithProofsRequest} request The request proto
 * @param {?Object<string, string>} metadata User defined
 *     call metadata
 * @return {!grpc.web.ClientReadableStream<!proto.org.dash.platform.dapi.v0.TransactionsWithProofsResponse>}
 *     The XHR Node Readable Stream
 */
proto.org.dash.platform.dapi.v0.CorePromiseClient.prototype.subscribeToTransactionsWithProofs =
    function(request, metadata) {
  return this.client_.serverStreaming(this.hostname_ +
      '/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs',
      request,
      metadata || {},
      methodDescriptor_Core_subscribeToTransactionsWithProofs);
};


module.exports = proto.org.dash.platform.dapi.v0;


const grpc = require('grpc');
const { promisify } = require('util');

const {
  convertObjectToMetadata,
  utils: {
    isObject,
  },
  client: {
    interceptors: {
      jsonToProtobufInterceptorFactory,
    },
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  org: {
    dash: {
      platform: {
        dapi: {
          v0: {
            GetStatusRequest: PBJSGetStatusRequest,
            GetStatusResponse: PBJSGetStatusResponse,
            GetBlockRequest: PBJSGetBlockRequest,
            GetBlockResponse: PBJSGetBlockResponse,
            SendTransactionRequest: PBJSSendTransactionRequest,
            SendTransactionResponse: PBJSSendTransactionResponse,
            GetTransactionRequest: PBJSGetTransactionRequest,
            GetTransactionResponse: PBJSGetTransactionResponse,
            BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
            BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
            GetEstimatedTransactionFeeRequest: PBJSGetEstimatedTransactionFeeRequest,
            GetEstimatedTransactionFeeResponse: PBJSGetEstimatedTransactionFeeResponse,
          },
        },
      },
    },
  },
} = require('./core_pbjs');

const {
  GetStatusResponse: ProtocGetStatusResponse,
  GetBlockResponse: ProtocGetBlockResponse,
  SendTransactionResponse: ProtocSendTransactionResponse,
  GetTransactionResponse: ProtocGetTransactionResponse,
  BlockHeadersWithChainLocksResponse: ProtocBlockHeadersWithChainLocksResponse,
  GetEstimatedTransactionFeeResponse: ProtocGetEstimatedTransactionFeeResponse,
} = require('./core_protoc');

const getCoreDefinition = require('../../lib/getCoreDefinition');
const stripHostname = require('../../lib/utils/stripHostname');

const CoreNodeJSClient = getCoreDefinition();

class CorePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    const strippedHostname = stripHostname(hostname);

    this.client = new CoreNodeJSClient(strippedHostname, credentials, options);

    this.client.getStatus = promisify(
      this.client.getStatus.bind(this.client),
    );

    this.client.getBlock = promisify(
      this.client.getBlock.bind(this.client),
    );

    this.client.sendTransaction = promisify(
      this.client.sendTransaction.bind(this.client),
    );

    this.client.getTransaction = promisify(
      this.client.getTransaction.bind(this.client),
    );

    this.client.getEstimatedTransactionFee = promisify(
      this.client.getEstimatedTransactionFee.bind(this.client),
    );

    this.protocolVersion = undefined;
  }

  /**
   * @param {!GetStatusRequest} getStatusRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetStatusResponse>}
   */
  getStatus(getStatusRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getStatus(
      getStatusRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetStatusResponse,
              PBJSGetStatusResponse,
            ),
            protobufToJsonFactory(
              PBJSGetStatusRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetBlockRequest} getBlockRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetBlockResponse>}
   */
  getBlock(getBlockRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getBlock(
      getBlockRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetBlockResponse,
              PBJSGetBlockResponse,
            ),
            protobufToJsonFactory(
              PBJSGetBlockRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!SendTransactionRequest} sendTransactionRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!SendTransactionResponse>}
   */
  sendTransaction(sendTransactionRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.sendTransaction(
      sendTransactionRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocSendTransactionResponse,
              PBJSSendTransactionResponse,
            ),
            protobufToJsonFactory(
              PBJSSendTransactionRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetTransactionRequest} getTransactionRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetTransactionResponse>}
   */
  getTransaction(getTransactionRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getTransaction(
      getTransactionRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetTransactionResponse,
              PBJSGetTransactionResponse,
            ),
            protobufToJsonFactory(
              PBJSGetTransactionRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetEstimatedTransactionFeeRequest} getEstimatedTransactionFeeRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetEstimatedTransactionFeeResponse>}
   */
  getEstimatedTransactionFee(getEstimatedTransactionFeeRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getEstimatedTransactionFee(
      getEstimatedTransactionFeeRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetEstimatedTransactionFeeResponse,
              PBJSGetEstimatedTransactionFeeResponse,
            ),
            protobufToJsonFactory(
              PBJSGetEstimatedTransactionFeeRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!BlockHeadersWithChainLocksRequest} blockHeadersWithChainLocksRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {!grpc.web.ClientReadableStream<!BlockHeadersWithChainLocksResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToBlockHeadersWithChainLocks(
    blockHeadersWithChainLocksRequest,
    metadata = {},
    options = {},
  ) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToBlockHeadersWithChainLocks(
      blockHeadersWithChainLocksRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocBlockHeadersWithChainLocksResponse,
              PBJSBlockHeadersWithChainLocksResponse,
            ),
            protobufToJsonFactory(
              PBJSBlockHeadersWithChainLocksRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {string} protocolVersion
   */
  setProtocolVersion(protocolVersion) {
    this.setProtocolVersion = protocolVersion;
  }
}

module.exports = CorePromiseClient;

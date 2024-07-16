const grpc = require('@grpc/grpc-js');
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

const { URL } = require('url');
const {
  org: {
    dash: {
      platform: {
        dapi: {
          v0: {
            GetBlockchainStatusRequest: PBJSGetBlockchainStatusRequest,
            GetBlockchainStatusResponse: PBJSGetBlockchainStatusResponse,
            GetMasternodeStatusRequest: PBJSGetMasternodeStatusRequest,
            GetMasternodeStatusResponse: PBJSGetMasternodeStatusResponse,
            GetBlockRequest: PBJSGetBlockRequest,
            GetBlockResponse: PBJSGetBlockResponse,
            GetBestBlockHeightRequest: PBJSGetBestBlockHeightRequest,
            GetBestBlockHeightResponse: PBJSGetBestBlockHeightResponse,
            BroadcastTransactionRequest: PBJSBroadcastTransactionRequest,
            BroadcastTransactionResponse: PBJSBroadcastTransactionResponse,
            GetTransactionRequest: PBJSGetTransactionRequest,
            GetTransactionResponse: PBJSGetTransactionResponse,
            BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
            BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
            GetEstimatedTransactionFeeRequest: PBJSGetEstimatedTransactionFeeRequest,
            GetEstimatedTransactionFeeResponse: PBJSGetEstimatedTransactionFeeResponse,
            TransactionsWithProofsRequest: PBJSTransactionsWithProofsRequest,
            TransactionsWithProofsResponse: PBJSTransactionsWithProofsResponse,
            MasternodeListRequest: PBJSMasternodeListRequest,
            MasternodeListResponse: PBJSMasternodeListResponse,
          },
        },
      },
    },
  },
} = require('./core_pbjs');

const {
  GetBlockchainStatusResponse: ProtocGetBlockchainStatusResponse,
  GetMasternodeStatusResponse: ProtocGetMasternodeStatusResponse,
  GetBlockResponse: ProtocGetBlockResponse,
  GetBestBlockHeightResponse: ProtocGetBestBlockHeightResponse,
  BroadcastTransactionResponse: ProtocBroadcastTransactionResponse,
  GetTransactionResponse: ProtocGetTransactionResponse,
  BlockHeadersWithChainLocksResponse: ProtocBlockHeadersWithChainLocksResponse,
  GetEstimatedTransactionFeeResponse: ProtocGetEstimatedTransactionFeeResponse,
  TransactionsWithProofsResponse: ProtocTransactionsWithProofsResponse,
  MasternodeListResponse: ProtocMasternodeListResponse,
} = require('./core_protoc');

const getCoreDefinition = require('../../../../lib/getCoreDefinition');

const CoreNodeJSClient = getCoreDefinition(0);

class CorePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials, options = {}) {
    if (credentials !== undefined) {
      throw new Error('"credentials" option is not supported yet');
    }

    const url = new URL(hostname);
    const { protocol, host: strippedHostname } = url;

    // See this issue https://github.com/nodejs/node/issues/3176
    // eslint-disable-next-line no-param-reassign
    credentials = protocol.replace(':', '') === 'https' ? grpc.credentials.createSsl() : grpc.credentials.createInsecure();

    this.client = new CoreNodeJSClient(strippedHostname, credentials, options);

    this.client.getBlockchainStatus = promisify(
      this.client.getBlockchainStatus.bind(this.client),
    );

    this.client.getMasternodeStatus = promisify(
      this.client.getMasternodeStatus.bind(this.client),
    );

    this.client.getBlock = promisify(
      this.client.getBlock.bind(this.client),
    );

    this.client.getBestBlockHeight = promisify(
      this.client.getBestBlockHeight.bind(this.client),
    );

    this.client.broadcastTransaction = promisify(
      this.client.broadcastTransaction.bind(this.client),
    );

    this.client.getTransaction = promisify(
      this.client.getTransaction.bind(this.client),
    );

    this.client.getEstimatedTransactionFee = promisify(
      this.client.getEstimatedTransactionFee.bind(this.client),
    );
  }

  /**
   * @param {!GetBlockchainStatusRequest} getBlockchainStatusRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetBlockchainStatusResponse>}
   */
  getBlockchainStatus(getBlockchainStatusRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getBlockchainStatus(
      getBlockchainStatusRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetBlockchainStatusResponse,
              PBJSGetBlockchainStatusResponse,
            ),
            protobufToJsonFactory(
              PBJSGetBlockchainStatusRequest,
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
   * @param {!GetBestBlockHeightRequest} getBestBlockHeightRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetBestBlockHeightResponse>}
   */
  getBestBlockHeight(getBestBlockHeightRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getBestBlockHeight(
      getBestBlockHeightRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetBestBlockHeightResponse,
              PBJSGetBestBlockHeightResponse,
            ),
            protobufToJsonFactory(
              PBJSGetBestBlockHeightRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetMasternodeStatusRequest} getMasternodeStatusRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetMasternodeStatusResponse>}
   */
  getMasternodeStatus(getMasternodeStatusRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getMasternodeStatus(
      getMasternodeStatusRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetMasternodeStatusResponse,
              PBJSGetMasternodeStatusResponse,
            ),
            protobufToJsonFactory(
              PBJSGetMasternodeStatusRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!BroadcastTransactionRequest} broadcastTransactionRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!BroadcastTransactionResponse>}
   */
  broadcastTransaction(broadcastTransactionRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.broadcastTransaction(
      broadcastTransactionRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocBroadcastTransactionResponse,
              PBJSBroadcastTransactionResponse,
            ),
            protobufToJsonFactory(
              PBJSBroadcastTransactionRequest,
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
   * @param {TransactionsWithProofsRequest} transactionsWithProofsRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @param {CallOptions} [options={}]
   * @return {!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToTransactionsWithProofs(transactionsWithProofsRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToTransactionsWithProofs(
      transactionsWithProofsRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocTransactionsWithProofsResponse,
              PBJSTransactionsWithProofsResponse,
            ),
            protobufToJsonFactory(
              PBJSTransactionsWithProofsRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {MasternodeListRequest} masternodeListRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @param {CallOptions} [options={}]
   * @return {!grpc.web.ClientReadableStream<!MasternodeListResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToMasternodeList(masternodeListRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToMasternodeList(
      masternodeListRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocMasternodeListResponse,
              PBJSMasternodeListResponse,
            ),
            protobufToJsonFactory(
              PBJSMasternodeListRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }
}

module.exports = CorePromiseClient;

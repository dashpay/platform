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
} = require('./core_protoc');

const getCoreDefinition = require('../../lib/getCoreDefinition');

const CoreNodeJSClient = getCoreDefinition();

const getStatusOptions = {
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
};

const getBlockOptions = {
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
};

const sendTransactionOptions = {
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
};

const getTransactionOptions = {
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
};

const subscribeToBlockHeadersWithChainLocksOptions = {
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
};

class CorePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    this.client = new CoreNodeJSClient(hostname, credentials, options);

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
  }

  /**
   * @param {!GetStatusRequest} getStatusRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetStatusResponse>}
   */
  getStatus(getStatusRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getStatus(
      getStatusRequest,
      convertObjectToMetadata(metadata),
      getStatusOptions,
    );
  }

  /**
   * @param {!GetBlockRequest} getBlockRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetBlockResponse>}
   */
  getBlock(getBlockRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getBlock(
      getBlockRequest,
      convertObjectToMetadata(metadata),
      getBlockOptions,
    );
  }

  /**
   * @param {!SendTransactionRequest} sendTransactionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!SendTransactionResponse>}
   */
  sendTransaction(sendTransactionRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.sendTransaction(
      sendTransactionRequest,
      convertObjectToMetadata(metadata),
      sendTransactionOptions,
    );
  }

  /**
   * @param {!GetTransactionRequest} getTransactionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetTransactionResponse>}
   */
  getTransaction(getTransactionRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getTransaction(
      getTransactionRequest,
      convertObjectToMetadata(metadata),
      getTransactionOptions,
    );
  }

  /**
   * @param {!BlockHeadersWithChainLocksRequest} blockHeadersWithChainLocksRequest
   * @param {?Object<string, string>} metadata
   * @return {!grpc.web.ClientReadableStream<!BlockHeadersWithChainLocksResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToBlockHeadersWithChainLocks(blockHeadersWithChainLocksRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToBlockHeadersWithChainLocks(
      blockHeadersWithChainLocksRequest,
      convertObjectToMetadata(metadata),
      subscribeToBlockHeadersWithChainLocksOptions,
    );
  }
}

module.exports = CorePromiseClient;

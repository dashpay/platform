const grpc = require('grpc');
const jsonToProtobufInterceptorFactory = require('../../src/jsonToProtobufInterceptorFactory');
const loadPackageDefinition = require('../../src/loadPackageDefinition');
const { BlockHeadersWithChainLocksResponse } = require('./core_pb');
const isObject = require('../../src/isObject');
const convertObjectToMetadata = require('../../src/convertObjectToMetadata');

const {
  Core: CoreNodeJSClient,
} = loadPackageDefinition('Core');

const subscribeToBlockHeadersWithChainLocksOptions = {
  interceptors: [jsonToProtobufInterceptorFactory(BlockHeadersWithChainLocksResponse)],
};

class CorePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    this.client = new CoreNodeJSClient(hostname, credentials, options);
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

    const message = blockHeadersWithChainLocksRequest.toObject();

    return this.client.subscribeToBlockHeadersWithChainLocks(
      message,
      convertObjectToMetadata(metadata),
      subscribeToBlockHeadersWithChainLocksOptions,
    );
  }
}

module.exports = CorePromiseClient;

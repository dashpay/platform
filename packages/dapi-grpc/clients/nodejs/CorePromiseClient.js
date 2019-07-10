const grpc = require('grpc');
const { promisify } = require('util');
const jsonToProtobufInterceptorFactory = require('../../src/interceptors/client/jsonToProtobufInterceptorFactory');
const loadPackageDefinition = require('../../src/loadPackageDefinition');

const {
  org: {
    dash: {
      platform: {
        dapi: {
          LastUserStateTransitionHashRequest: PBJSLastUserStateTransitionHashRequest,
          LastUserStateTransitionHashResponse: PBJSLastUserStateTransitionHashResponse,
          BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
          BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
        },
      },
    },
  },
} = require('./core_pbjs');

const {
  LastUserStateTransitionHashResponse: ProtocLastUserStateTransitionHashResponse,
  BlockHeadersWithChainLocksResponse: ProtocBlockHeadersWithChainLocksResponse,
} = require('./core_protoc');

const isObject = require('../../src/isObject');
const convertObjectToMetadata = require('../../src/convertObjectToMetadata');

const jsonToProtobufFactory = require('../../src/converters/jsonToProtobufFactory');
const protobufToJsonFactory = require('../../src/converters/protobufToJsonFactory');

const {
  Core: CoreNodeJSClient,
} = loadPackageDefinition('Core');

const getLastUserStateTransitionHashOptions = {
  interceptors: [
    jsonToProtobufInterceptorFactory(
      jsonToProtobufFactory(
        ProtocLastUserStateTransitionHashResponse,
        PBJSLastUserStateTransitionHashResponse,
      ),
      protobufToJsonFactory(
        PBJSLastUserStateTransitionHashRequest,
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

    this.client.getLastUserStateTransitionHash = promisify(
      this.client.getLastUserStateTransitionHash.bind(this.client),
    );
  }

  /**
   * @param {!LastUserStateTransitionHashRequest} lastUserStateTransitionHashRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!LastUserStateTransitionHashResponse>}
   */
  getLastUserStateTransitionHash(lastUserStateTransitionHashRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getLastUserStateTransitionHash(
      lastUserStateTransitionHashRequest,
      convertObjectToMetadata(metadata),
      getLastUserStateTransitionHashOptions,
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

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
          LastUserStateTransitionHashRequest: PBJSLastUserStateTransitionHashRequest,
          LastUserStateTransitionHashResponse: PBJSLastUserStateTransitionHashResponse,
          BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
          BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
          StateTransition: PBJSStateTransition,
          UpdateStateTransitionResponse: PBJSUpdateStateTransitionResponse,
        },
      },
    },
  },
} = require('./core_pbjs');

const {
  LastUserStateTransitionHashResponse: ProtocLastUserStateTransitionHashResponse,
  BlockHeadersWithChainLocksResponse: ProtocBlockHeadersWithChainLocksResponse,
  UpdateStateTransitionResponse: ProtocUpdateStateTransitionResponse,
} = require('./core_protoc');

const getCoreDefinition = require('../../lib/getCoreDefinition');

const CoreNodeJSClient = getCoreDefinition();

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

const updateStateTransitionOptions = {
  interceptors: [
    jsonToProtobufInterceptorFactory(
      jsonToProtobufFactory(
        ProtocUpdateStateTransitionResponse,
        PBJSUpdateStateTransitionResponse,
      ),
      protobufToJsonFactory(
        PBJSStateTransition,
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

    this.client.updateState = promisify(
      this.client.updateState.bind(this.client),
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

  /**
   *
   * @param {!StateTransition} stateTransition
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!UpdateStateTransitionResponse>}
   */
  updateState(stateTransition, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.updateState(
      stateTransition,
      convertObjectToMetadata(metadata),
      updateStateTransitionOptions,
    );
  }
}

module.exports = CorePromiseClient;

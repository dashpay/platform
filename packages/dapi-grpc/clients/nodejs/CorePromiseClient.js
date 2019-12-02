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
            LastUserStateTransitionHashRequest: PBJSLastUserStateTransitionHashRequest,
            LastUserStateTransitionHashResponse: PBJSLastUserStateTransitionHashResponse,
            BlockHeadersWithChainLocksRequest: PBJSBlockHeadersWithChainLocksRequest,
            BlockHeadersWithChainLocksResponse: PBJSBlockHeadersWithChainLocksResponse,
            UpdateStateRequest: PBJSUpdateStateRequest,
            UpdateStateResponse: PBJSUpdateStateResponse,
            FetchIdentityRequest: PBJSFetchIdentityRequest,
            FetchIdentityResponse: PBJSFetchIdentityResponse,
          },
        },
      },
    },
  },
} = require('./core_pbjs');

const {
  LastUserStateTransitionHashResponse: ProtocLastUserStateTransitionHashResponse,
  BlockHeadersWithChainLocksResponse: ProtocBlockHeadersWithChainLocksResponse,
  UpdateStateResponse: ProtocUpdateStateResponse,
  FetchIdentityResponse: ProtocFetchIdentityResponse,
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
        ProtocUpdateStateResponse,
        PBJSUpdateStateResponse,
      ),
      protobufToJsonFactory(
        PBJSUpdateStateRequest,
      ),
    ),
  ],
};

const fetchIdentityOptions = {
  interceptors: [
    jsonToProtobufInterceptorFactory(
      jsonToProtobufFactory(
        ProtocFetchIdentityResponse,
        PBJSFetchIdentityResponse,
      ),
      protobufToJsonFactory(
        PBJSFetchIdentityRequest,
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
   * @param {!UpdateStateRequest} updateStateRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!UpdateStateResponse>}
   */
  updateState(updateStateRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.updateState(
      updateStateRequest,
      convertObjectToMetadata(metadata),
      updateStateTransitionOptions,
    );
  }

  /**
   *
   * @param {!FetchIdentityRequest} fetchIdentityRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!FetchIdentityResponse>}
   */
  fetchIdentity(fetchIdentityRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.fetchIdentity(
      fetchIdentityRequest,
      convertObjectToMetadata(metadata),
      fetchIdentityOptions,
    );
  }
}

module.exports = CorePromiseClient;

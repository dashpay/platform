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
            BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
            BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
            GetIdentityRequest: PBJSGetIdentityRequest,
            GetIdentityResponse: PBJSGetIdentityResponse,
            GetDataContractRequest: PBJSGetDataContractRequest,
            GetDataContractResponse: PBJSGetDataContractResponse,
            GetDocumentsRequest: PBJSGetDocumentsRequest,
            GetDocumentsResponse: PBJSGetDocumentsResponse,
            GetIdentityByFirstPublicKeyRequest: PBJSGetIdentityByFirstPublicKeyRequest,
            GetIdentityByFirstPublicKeyResponse: PBJSGetIdentityByFirstPublicKeyResponse,
            GetIdentityIdByFirstPublicKeyRequest: PBJSGetIdentityIdByFirstPublicKeyRequest,
            GetIdentityIdByFirstPublicKeyResponse: PBJSGetIdentityIdByFirstPublicKeyResponse,
            GetIdentitiesByPublicKeyHashesRequest: PBJSGetIdentitiesByPublicKeyHashesRequest,
            GetIdentitiesByPublicKeyHashesResponse: PBJSGetIdentitiesByPublicKeyHashesResponse,
            GetIdentityIdsByPublicKeyHashesRequest: PBJSGetIdentityIdsByPublicKeyHashesRequest,
            GetIdentityIdsByPublicKeyHashesResponse: PBJSGetIdentityIdsByPublicKeyHashesResponse,
          },
        },
      },
    },
  },
} = require('./platform_pbjs');

const {
  BroadcastStateTransitionResponse: ProtocBroadcastStateTransitionResponse,
  GetIdentityResponse: ProtocGetIdentityResponse,
  GetDataContractResponse: ProtocGetDataContractResponse,
  GetDocumentsResponse: ProtocGetDocumentsResponse,
  GetIdentityByFirstPublicKeyResponse: ProtocGetIdentityByFirstPublicKeyResponse,
  GetIdentityIdByFirstPublicKeyResponse: ProtocGetIdentityIdByFirstPublicKeyResponse,
  GetIdentitiesByPublicKeyHashesResponse: ProtocGetIdentitiesByPublicKeyHashesResponse,
  GetIdentityIdsByPublicKeyHashesResponse: ProtocGetIdentityIdsByPublicKeyHashesResponse,
} = require('./platform_protoc');

const getPlatformDefinition = require('../../../../lib/getPlatformDefinition');
const stripHostname = require('../../../../lib/utils/stripHostname');

const PlatformNodeJSClient = getPlatformDefinition(0);

class PlatformPromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    const strippedHostname = stripHostname(hostname);

    this.client = new PlatformNodeJSClient(strippedHostname, credentials, options);

    this.client.broadcastStateTransition = promisify(
      this.client.broadcastStateTransition.bind(this.client),
    );

    this.client.getIdentity = promisify(
      this.client.getIdentity.bind(this.client),
    );

    this.client.getDataContract = promisify(
      this.client.getDataContract.bind(this.client),
    );

    this.client.getDocuments = promisify(
      this.client.getDocuments.bind(this.client),
    );

    this.client.getIdentityByFirstPublicKey = promisify(
      this.client.getIdentityByFirstPublicKey.bind(this.client),
    );

    this.client.getIdentityIdByFirstPublicKey = promisify(
      this.client.getIdentityIdByFirstPublicKey.bind(this.client),
    );

    this.client.getIdentitiesByPublicKeyHashes = promisify(
      this.client.getIdentitiesByPublicKeyHashes.bind(this.client),
    );

    this.client.getIdentityIdsByPublicKeyHashes = promisify(
      this.client.getIdentityIdsByPublicKeyHashes.bind(this.client),
    );

    this.protocolVersion = undefined;
  }

  /**
   * @param {!BroadcastStateTransitionRequest} broadcastStateTransitionRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!BroadcastStateTransitionResponse>}
   */
  broadcastStateTransition(broadcastStateTransitionRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.broadcastStateTransition(
      broadcastStateTransitionRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocBroadcastStateTransitionResponse,
              PBJSBroadcastStateTransitionResponse,
            ),
            protobufToJsonFactory(
              PBJSBroadcastStateTransitionRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetIdentityRequest} getIdentityRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetIdentityResponse>}
   */
  getIdentity(getIdentityRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getIdentity(
      getIdentityRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetIdentityResponse,
              PBJSGetIdentityResponse,
            ),
            protobufToJsonFactory(
              PBJSGetIdentityRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   *
   * @param {!GetDataContractRequest} getDataContractRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetDataContractResponse>}
   */
  getDataContract(getDataContractRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getDataContract(
      getDataContractRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetDataContractResponse,
              PBJSGetDataContractResponse,
            ),
            protobufToJsonFactory(
              PBJSGetDataContractRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   *
   * @param {!GetDocumentsRequest} getDocumentsRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetDocumentsResponse>}
   */
  getDocuments(getDocumentsRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getDocuments(
      getDocumentsRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetDocumentsResponse,
              PBJSGetDocumentsResponse,
            ),
            protobufToJsonFactory(
              PBJSGetDocumentsRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @deprecated
   * @param {!GetIdentityByFirstPublicKeyRequest} getIdentityByFirstPublicKeyRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetIdentityByFirstPublicKeyResponse>}
   */
  getIdentityByFirstPublicKey(getIdentityByFirstPublicKeyRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getIdentityByFirstPublicKey(
      getIdentityByFirstPublicKeyRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetIdentityByFirstPublicKeyResponse,
              PBJSGetIdentityByFirstPublicKeyResponse,
            ),
            protobufToJsonFactory(
              PBJSGetIdentityByFirstPublicKeyRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @deprecated
   * @param {!GetIdentityIdByFirstPublicKeyRequest} getIdentityIdByFirstPublicKeyRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetIdentityIdByFirstPublicKeyResponse>}
   */
  getIdentityIdByFirstPublicKey(getIdentityIdByFirstPublicKeyRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getIdentityIdByFirstPublicKey(
      getIdentityIdByFirstPublicKeyRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetIdentityIdByFirstPublicKeyResponse,
              PBJSGetIdentityIdByFirstPublicKeyResponse,
            ),
            protobufToJsonFactory(
              PBJSGetIdentityIdByFirstPublicKeyRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetIdentitiesByPublicKeyHashesRequest} getIdentitiesByPublicKeyHashesRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetIdentitiesByPublicKeyHashesResponse>}
   */
  getIdentitiesByPublicKeyHashes(
    getIdentitiesByPublicKeyHashesRequest, metadata = {}, options = {},
  ) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getIdentitiesByPublicKeyHashes(
      getIdentitiesByPublicKeyHashesRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetIdentitiesByPublicKeyHashesResponse,
              PBJSGetIdentitiesByPublicKeyHashesResponse,
            ),
            protobufToJsonFactory(
              PBJSGetIdentitiesByPublicKeyHashesRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetIdentityIdsByPublicKeyHashesRequest} getIdentityIdsByPublicKeyHashesRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetIdentityIdsByPublicKeyHashesResponse>}
   */
  getIdentityIdsByPublicKeyHashes(
    getIdentityIdsByPublicKeyHashesRequest, metadata = {}, options = {},
  ) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getIdentityIdsByPublicKeyHashes(
      getIdentityIdsByPublicKeyHashesRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetIdentityIdsByPublicKeyHashesResponse,
              PBJSGetIdentityIdsByPublicKeyHashesResponse,
            ),
            protobufToJsonFactory(
              PBJSGetIdentityIdsByPublicKeyHashesRequest,
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

module.exports = PlatformPromiseClient;

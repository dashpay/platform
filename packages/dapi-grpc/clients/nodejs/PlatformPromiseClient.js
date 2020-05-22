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
            ApplyStateTransitionRequest: PBJSApplyStateTransitionRequest,
            ApplyStateTransitionResponse: PBJSApplyStateTransitionResponse,
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
          },
        },
      },
    },
  },
} = require('./platform_pbjs');

const {
  ApplyStateTransitionResponse: ProtocApplyStateTransitionResponse,
  GetIdentityResponse: ProtocGetIdentityResponse,
  GetDataContractResponse: ProtocGetDataContractResponse,
  GetDocumentsResponse: ProtocGetDocumentsResponse,
  GetIdentityByFirstPublicKeyResponse: ProtocGetIdentityByFirstPublicKeyResponse,
  GetIdentityIdByFirstPublicKeyResponse: ProtocGetIdentityIdByFirstPublicKeyResponse,
} = require('./platform_protoc');

const getPlatformDefinition = require('../../lib/getPlatformDefinition');

const PlatformNodeJSClient = getPlatformDefinition();

class PlatformPromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    this.client = new PlatformNodeJSClient(hostname, credentials, options);

    this.client.applyStateTransition = promisify(
      this.client.applyStateTransition.bind(this.client),
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

    this.protocolVersion = undefined;
  }

  /**
   * @param {!ApplyStateTransitionRequest} applyStateTransitionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!ApplyStateTransitionResponse>}
   */
  applyStateTransition(applyStateTransitionRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.applyStateTransition(
      applyStateTransitionRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocApplyStateTransitionResponse,
              PBJSApplyStateTransitionResponse,
            ),
            protobufToJsonFactory(
              PBJSApplyStateTransitionRequest,
            ),
          ),
        ],
      },
    );
  }

  /**
   * @param {!GetIdentityRequest} getIdentityRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetIdentityResponse>}
   */
  getIdentity(getIdentityRequest, metadata = {}) {
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
      },
    );
  }

  /**
   *
   * @param {!GetDataContractRequest} getDataContractRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDataContractResponse>}
   */
  getDataContract(getDataContractRequest, metadata = {}) {
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
      },
    );
  }

  /**
   *
   * @param {!GetDocumentsRequest} getDocumentsRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDocumentsResponse>}
   */
  getDocuments(getDocumentsRequest, metadata = {}) {
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
      },
    );
  }

  /**
   * @param {string} protocolVersion
   */
  setProtocolVersion(protocolVersion) {
    this.setProtocolVersion = protocolVersion;
  }

  /**
   *
   * @param {!GetIdentityByFirstPublicKeyRequest} getIdentityByFirstPublicKeyRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDocumentsResponse>}
   */
  getIdentityByFirstPublicKey(getIdentityByFirstPublicKeyRequest, metadata = {}) {
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
      },
    );
  }


  /**
   *
   * @param {!GetIdentityIdByFirstPublicKeyRequest} getIdentityIdByFirstPublicKeyRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDocumentsResponse>}
   */
  getIdentityIdByFirstPublicKey(getIdentityIdByFirstPublicKeyRequest, metadata = {}) {
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
      },
    );
  }
}

module.exports = PlatformPromiseClient;

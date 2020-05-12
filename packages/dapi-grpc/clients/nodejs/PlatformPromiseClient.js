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
      protocolVersionInterceptorFactory,
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
  GetDocumentsResponse: ProtocGetDocumentsRequest,
} = require('./platform_protoc');

const getPlatformDefinition = require('../../lib/getPlatformDefinition');

const PlatformNodeJSClient = getPlatformDefinition();

class PlatformPromiseClient {
  /**
   * @param {string} hostname
   * @param {string} version
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
          protocolVersionInterceptorFactory(this.protocolVersion),
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
          protocolVersionInterceptorFactory(this.protocolVersion),
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
          protocolVersionInterceptorFactory(this.protocolVersion),
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
          protocolVersionInterceptorFactory(this.protocolVersion),
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetDocumentsRequest,
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
}

module.exports = PlatformPromiseClient;

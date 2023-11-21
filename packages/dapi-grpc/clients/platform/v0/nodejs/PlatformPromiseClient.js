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
            BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
            BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
            GetIdentityRequest: PBJSGetIdentityRequest,
            GetIdentityResponse: PBJSGetIdentityResponse,
            GetDataContractRequest: PBJSGetDataContractRequest,
            GetDataContractResponse: PBJSGetDataContractResponse,
            GetDataContractHistoryRequest: PBJSGetDataContractHistoryRequest,
            GetDataContractHistoryResponse: PBJSGetDataContractHistoryResponse,
            GetDocumentsRequest: PBJSGetDocumentsRequest,
            GetDocumentsResponse: PBJSGetDocumentsResponse,
            GetIdentitiesByPublicKeyHashesRequest: PBJSGetIdentitiesByPublicKeyHashesRequest,
            GetIdentitiesByPublicKeyHashesResponse: PBJSGetIdentitiesByPublicKeyHashesResponse,
            WaitForStateTransitionResultRequest: PBJSWaitForStateTransitionResultRequest,
            WaitForStateTransitionResultResponse: PBJSWaitForStateTransitionResultResponse,
            GetConsensusParamsRequest: PBJSGetConsensusParamsRequest,
            GetConsensusParamsResponse: PBJSGetConsensusParamsResponse,
            GetEpochsInfoRequest: PBJSGetEpochsInfoRequest,
            GetEpochsInfoResponse: PBJSGetEpochsInfoResponse,
            GetProtocolVersionUpgradeVoteStatusRequest:
              PBJSGetProtocolVersionUpgradeVoteStatusRequest,
            GetProtocolVersionUpgradeVoteStatusResponse:
              PBJSGetProtocolVersionUpgradeVoteStatusResponse,
            GetProtocolVersionUpgradeStateRequest: PBJSGetProtocolVersionUpgradeStateRequest,
            GetProtocolVersionUpgradeStateResponse: PBJSGetProtocolVersionUpgradeStateResponse,
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
  GetDataContractHistoryResponse: ProtocGetDataContractHistoryResponse,
  GetDocumentsResponse: ProtocGetDocumentsResponse,
  GetIdentitiesByPublicKeyHashesResponse: ProtocGetIdentitiesByPublicKeyHashesResponse,
  WaitForStateTransitionResultResponse: ProtocWaitForStateTransitionResultResponse,
  GetConsensusParamsResponse: ProtocGetConsensusParamsResponse,
  GetEpochsInfoResponse: ProtocGetEpochsInfoResponse,
  GetProtocolVersionUpgradeVoteStatusResponse: ProtocGetProtocolVersionUpgradeVoteStatusResponse,
  GetProtocolVersionUpgradeStateResponse: ProtocGetProtocolVersionUpgradeStateResponse,
} = require('./platform_protoc');

const getPlatformDefinition = require('../../../../lib/getPlatformDefinition');

const PlatformNodeJSClient = getPlatformDefinition(0);

class PlatformPromiseClient {
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

    this.client.getDataContractHistory = promisify(
      this.client.getDataContractHistory.bind(this.client),
    );

    this.client.getDocuments = promisify(
      this.client.getDocuments.bind(this.client),
    );

    this.client.getIdentitiesByPublicKeyHashes = promisify(
      this.client.getIdentitiesByPublicKeyHashes.bind(this.client),
    );

    this.client.waitForStateTransitionResult = promisify(
      this.client.waitForStateTransitionResult.bind(this.client),
    );

    this.client.getConsensusParams = promisify(
      this.client.getConsensusParams.bind(this.client),
    );

    this.client.getEpochsInfo = promisify(
      this.client.getEpochsInfo.bind(this.client),
    );

    this.client.getProtocolVersionUpgradeVoteStatus = promisify(
      this.client.getProtocolVersionUpgradeVoteStatus.bind(this.client),
    );

    this.client.getProtocolVersionUpgradeState = promisify(
      this.client.getProtocolVersionUpgradeState.bind(this.client),
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
   * @param {!GetDataContractHistoryRequest} getDataContractHistoryRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetDataContractResponse>}
   */
  getDataContractHistory(getDataContractHistoryRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getDataContractHistory(
      getDataContractHistoryRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetDataContractHistoryResponse,
              PBJSGetDataContractHistoryResponse,
            ),
            protobufToJsonFactory(
              PBJSGetDataContractHistoryRequest,
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
   * @param {!GetIdentitiesByPublicKeyHashesRequest} getIdentitiesByPublicKeyHashesRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetIdentitiesByPublicKeyHashesResponse>}
   */
  getIdentitiesByPublicKeyHashes(
    getIdentitiesByPublicKeyHashesRequest,
    metadata = {},
    options = {},
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
   * @param {!WaitForStateTransitionResultRequest} waitForStateTransitionResultRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!WaitForStateTransitionResultResponse>}
   */
  waitForStateTransitionResult(waitForStateTransitionResultRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.waitForStateTransitionResult(
      waitForStateTransitionResultRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocWaitForStateTransitionResultResponse,
              PBJSWaitForStateTransitionResultResponse,
            ),
            protobufToJsonFactory(
              PBJSWaitForStateTransitionResultRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetConsensusParamsRequest} getConsensusParamsRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetConsensusParamsResponse>}
   */
  getConsensusParams(getConsensusParamsRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getConsensusParams(
      getConsensusParamsRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetConsensusParamsResponse,
              PBJSGetConsensusParamsResponse,
            ),
            protobufToJsonFactory(
              PBJSGetConsensusParamsRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetEpochsInfoRequest} getEpochsInfoRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetEpochsInfoResponse>}
   */
  getEpochsInfo(getEpochsInfoRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getEpochsInfo(
      getEpochsInfoRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetEpochsInfoResponse,
              PBJSGetEpochsInfoResponse,
            ),
            protobufToJsonFactory(
              PBJSGetEpochsInfoRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetProtocolVersionUpgradeVoteStatusRequest} getProtocolVersionUpgradeVoteStatusRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetProtocolVersionUpgradeVoteStatusResponse>}
   */
  getProtocolVersionUpgradeVoteStatus(
    getProtocolVersionUpgradeVoteStatusRequest,
    metadata = {},
    options = {},
  ) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getProtocolVersionUpgradeVoteStatus(
      getProtocolVersionUpgradeVoteStatusRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetProtocolVersionUpgradeVoteStatusResponse,
              PBJSGetProtocolVersionUpgradeVoteStatusResponse,
            ),
            protobufToJsonFactory(
              PBJSGetProtocolVersionUpgradeVoteStatusRequest,
            ),
          ),
        ],
        ...options,
      },
    );
  }

  /**
   * @param {!GetProtocolVersionUpgradeStateRequest} getProtocolVersionUpgradeStateRequest
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @return {Promise<!GetProtocolVersionUpgradeStateResponse>}
   */
  getProtocolVersionUpgradeState(
    getProtocolVersionUpgradeStateRequest,
    metadata = {},
    options = {},
  ) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getProtocolVersionUpgradeState(
      getProtocolVersionUpgradeStateRequest,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetProtocolVersionUpgradeStateResponse,
              PBJSGetProtocolVersionUpgradeStateResponse,
            ),
            protobufToJsonFactory(
              PBJSGetProtocolVersionUpgradeStateRequest,
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

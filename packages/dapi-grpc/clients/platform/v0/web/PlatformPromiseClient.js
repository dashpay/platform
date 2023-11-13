const { PlatformClient } = require('./platform_pb_service');
const { promisify } = require('util');

class PlatformPromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials , options = {}) {
    this.client = new PlatformClient(hostname, options)

    this.protocolVersion = undefined;
  }

  /**
   * @param {!BroadcastStateTransitionRequest} broadcastStateTransitionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!BroadcastStateTransitionResponse>}
   */
  broadcastStateTransition(broadcastStateTransitionRequest, metadata = {}) {
    return promisify(
      this.client.broadcastStateTransition.bind(this.client),
    )(
      broadcastStateTransitionRequest,
      metadata,
    );
  }

  /**
   * @param {!GetIdentityRequest} getIdentityRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetIdentityResponse>}
   */
  getIdentity(getIdentityRequest, metadata = {}) {
    return promisify(
      this.client.getIdentity.bind(this.client),
    )(
      getIdentityRequest,
      metadata,
    );
  }

  /**
   *
   * @param {!GetDataContractRequest} getDataContractRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDataContractResponse>}
   */
  getDataContract(getDataContractRequest, metadata = {}) {
    return promisify(
      this.client.getDataContract.bind(this.client),
    )(
      getDataContractRequest,
      metadata,
    );
  }

  /**
   *
   * @param {!GetDataContractHistoryRequest} getDataContractHistoryRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDataContractResponse>}
   */
  getDataContractHistory(getDataContractHistoryRequest, metadata = {}) {
    return promisify(
        this.client.getDataContractHistory.bind(this.client),
    )(
        getDataContractHistoryRequest,
        metadata,
    );
  }

  /**
   *
   * @param {!GetDocumentsRequest} getDocumentsRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetDocumentsResponse>}
   */
  getDocuments(getDocumentsRequest, metadata = {}) {
    return promisify(
      this.client.getDocuments.bind(this.client),
    )(
      getDocumentsRequest,
      metadata,
    );
  }

  /**
   * @param {!GetIdentitiesByPublicKeyHashesRequest} getIdentitiesByPublicKeyHashesRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetIdentitiesByPublicKeyHashesResponse>}
   */
  getIdentitiesByPublicKeyHashes(
    getIdentitiesByPublicKeyHashesRequest, metadata = {}
  ) {
    return promisify(
      this.client.getIdentitiesByPublicKeyHashes.bind(this.client),
    )(
      getIdentitiesByPublicKeyHashesRequest,
      metadata,
    );
  }

  /**
   * @param {!WaitForStateTransitionResultRequest} waitForStateTransitionResultRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!WaitForStateTransitionResultResponse>}
   */
  waitForStateTransitionResult(
    waitForStateTransitionResultRequest, metadata = {}
  ) {
    return promisify(
      this.client.waitForStateTransitionResult.bind(this.client),
    )(
      waitForStateTransitionResultRequest,
      metadata,
    );
  }

  /**
   * @param {!GetConsensusParamsRequest} getConsensusParamsRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetConsensusParamsResponse>}
   */
  getConsensusParams(
    getConsensusParamsRequest, metadata = {}
  ) {
    return promisify(
      this.client.getConsensusParams.bind(this.client),
    )(
      getConsensusParamsRequest,
      metadata,
    );
  }

  /**
   * @param {!GetEpochsInfoRequest} getEpochsInfoRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetEpochsInfoResponse>}
   */
  getEpochsInfo(getEpochsInfoRequest, metadata = {}) {
    return promisify(
      this.client.getEpochsInfo.bind(this.client),
    )(
      getEpochsInfoRequest,
      metadata,
    );
  }

  /**
   * @param {!GetProtocolVersionUpgradeVoteStatusRequest} getProtocolVersionUpgradeVoteStatusRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetProtocolVersionUpgradeVoteStatusResponse>}
   */
  getProtocolVersionUpgradeVoteStatus(getProtocolVersionUpgradeVoteStatusRequest, metadata = {}) {
    return promisify(
      this.client.getProtocolVersionUpgradeVoteStatus.bind(this.client),
    )(
      getProtocolVersionUpgradeVoteStatusRequest,
      metadata,
    );
  }

  /**
   * @param {!GetProtocolVersionUpgradeStateRequest} getProtocolVersionUpgradeStateRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetProtocolVersionUpgradeStateResponse>}
   */
  getProtocolVersionUpgradeState(getProtocolVersionUpgradeStateRequest, metadata = {}) {
    return promisify(
      this.client.getProtocolVersionUpgradeState.bind(this.client),
    )(
      getProtocolVersionUpgradeStateRequest,
      metadata,
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

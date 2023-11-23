const broadcastStateTransitionFactory = require('./broadcastStateTransition/broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContract/getDataContractFactory');
const getDataContractHistoryFactory = require('./getDataContractHistory/getDataContractHistoryFactory');
const getDocumentsFactory = require('./getDocuments/getDocumentsFactory');
const getIdentityFactory = require('./getIdentity/getIdentityFactory');
const getIdentitiesByPublicKeyHashesFactory = require('./getIdentitiesByPublicKeyHashes/getIdentitiesByPublicKeyHashesFactory');
const waitForStateTransitionResultFactory = require('./waitForStateTransitionResult/waitForStateTransitionResultFactory');
const getConsensusParamsFactory = require('./getConsensusParams/getConsensusParamsFactory');
const getEpochsInfoFactory = require('./getEpochsInfo/getEpochsInfoFactory');
const getProtocolVersionUpgradeVoteStatusFactory = require('./getProtocolVersionUpgradeVoteStatus/getProtocolVersionUpgradeVoteStatusFactory');
const getProtocolVersionUpgradeStateFactory = require('./getProtocolVersionUpgradeState/getProtocolVersionUpgradeStateFactory');

class PlatformMethodsFacade {
  /**
   * @param {GrpcTransport} grpcTransport
   */
  constructor(grpcTransport) {
    this.broadcastStateTransition = broadcastStateTransitionFactory(grpcTransport);
    this.getDataContract = getDataContractFactory(grpcTransport);
    this.getDataContractHistory = getDataContractHistoryFactory(grpcTransport);
    this.getDocuments = getDocumentsFactory(grpcTransport);
    this.getIdentity = getIdentityFactory(grpcTransport);
    this.getIdentitiesByPublicKeyHashes = getIdentitiesByPublicKeyHashesFactory(grpcTransport);
    this.waitForStateTransitionResult = waitForStateTransitionResultFactory(grpcTransport);
    this.getConsensusParams = getConsensusParamsFactory(grpcTransport);
    this.getEpochsInfo = getEpochsInfoFactory(grpcTransport);
    this.getProtocolVersionUpgradeVoteStatus = getProtocolVersionUpgradeVoteStatusFactory(
      grpcTransport,
    );
    this.getProtocolVersionUpgradeState = getProtocolVersionUpgradeStateFactory(grpcTransport);
  }
}

module.exports = PlatformMethodsFacade;

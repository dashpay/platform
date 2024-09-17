const broadcastStateTransitionFactory = require('./broadcastStateTransition/broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContract/getDataContractFactory');
const getDataContractHistoryFactory = require('./getDataContractHistory/getDataContractHistoryFactory');
const getDocumentsFactory = require('./getDocuments/getDocumentsFactory');
const getIdentityFactory = require('./getIdentity/getIdentityFactory');
const getIdentityByPublicKeyHashFactory = require('./getIdentityByPublicKeyHash/getIdentityByPublicKeyHashFactory');
const getIdentitiesContractKeysFactory = require('./getIdentitiesContractKeys/getIdentitiesContractKeysFactory');
const waitForStateTransitionResultFactory = require('./waitForStateTransitionResult/waitForStateTransitionResultFactory');
const getConsensusParamsFactory = require('./getConsensusParams/getConsensusParamsFactory');
const getEpochsInfoFactory = require('./getEpochsInfo/getEpochsInfoFactory');
const getProtocolVersionUpgradeVoteStatusFactory = require('./getProtocolVersionUpgradeVoteStatus/getProtocolVersionUpgradeVoteStatusFactory');
const getProtocolVersionUpgradeStateFactory = require('./getProtocolVersionUpgradeState/getProtocolVersionUpgradeStateFactory');
const getIdentityContractNonceFactory = require('./getIdentityContractNonce/getIdentityContractNonceFactory');
const getIdentityNonceFactory = require('./getIdentityNonce/getIdentityNonceFactory');
const getIdentityKeysFactory = require('./getIdentityKeys/getIdentityKeysFactory');
const getTotalCreditsInPlatformFactory = require('./getTotalCreditsInPlatform/getTotalCreditsInPlatformFactory');
const getStatusFactory = require('./getStatus/getStatusFactory');
const getIdentityBalanceFactory = require('./getIdentityBalance/getIdentityBalanceFactory');

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
    this.getIdentityByPublicKeyHash = getIdentityByPublicKeyHashFactory(grpcTransport);
    this.getIdentitiesContractKeys = getIdentitiesContractKeysFactory(grpcTransport);
    this.waitForStateTransitionResult = waitForStateTransitionResultFactory(grpcTransport);
    this.getConsensusParams = getConsensusParamsFactory(grpcTransport);
    this.getEpochsInfo = getEpochsInfoFactory(grpcTransport);
    this.getProtocolVersionUpgradeVoteStatus = getProtocolVersionUpgradeVoteStatusFactory(
      grpcTransport,
    );
    this.getProtocolVersionUpgradeState = getProtocolVersionUpgradeStateFactory(grpcTransport);
    this.getIdentityContractNonce = getIdentityContractNonceFactory(grpcTransport);
    this.getIdentityNonce = getIdentityNonceFactory(grpcTransport);
    this.getIdentityKeys = getIdentityKeysFactory(grpcTransport);
    this.getTotalCreditsInPlatform = getTotalCreditsInPlatformFactory(grpcTransport);
    this.getStatus = getStatusFactory(grpcTransport);
    this.getIdentityBalance = getIdentityBalanceFactory(grpcTransport);
  }
}

module.exports = PlatformMethodsFacade;

import broadcastStateTransitionFactory from './broadcastStateTransition/broadcastStateTransitionFactory.js';
import getDataContractFactory from './getDataContract/getDataContractFactory.js';
import getDataContractHistoryFactory from './getDataContractHistory/getDataContractHistoryFactory.js';
import getDocumentsFactory from './getDocuments/getDocumentsFactory.js';
import getIdentityFactory from './getIdentity/getIdentityFactory.js';
import getIdentityByPublicKeyHashFactory from './getIdentityByPublicKeyHash/getIdentityByPublicKeyHashFactory.js';
import getIdentitiesContractKeysFactory from './getIdentitiesContractKeys/getIdentitiesContractKeysFactory.js';
import waitForStateTransitionResultFactory from './waitForStateTransitionResult/waitForStateTransitionResultFactory.js';
import getConsensusParamsFactory from './getConsensusParams/getConsensusParamsFactory.js';
import getEpochsInfoFactory from './getEpochsInfo/getEpochsInfoFactory.js';
import getProtocolVersionUpgradeVoteStatusFactory from './getProtocolVersionUpgradeVoteStatus/getProtocolVersionUpgradeVoteStatusFactory.js';
import getProtocolVersionUpgradeStateFactory from './getProtocolVersionUpgradeState/getProtocolVersionUpgradeStateFactory.js';
import getIdentityContractNonceFactory from './getIdentityContractNonce/getIdentityContractNonceFactory.js';
import getIdentityNonceFactory from './getIdentityNonce/getIdentityNonceFactory.js';
import getIdentityKeysFactory from './getIdentityKeys/getIdentityKeysFactory.js';
import getTotalCreditsInPlatformFactory from './getTotalCreditsInPlatform/getTotalCreditsInPlatformFactory.js';
import getStatusFactory from './getStatus/getStatusFactory.js';
import getIdentityBalanceFactory from './getIdentityBalance/getIdentityBalanceFactory.js';

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

export default PlatformMethodsFacade;

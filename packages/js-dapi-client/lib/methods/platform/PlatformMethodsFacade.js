const broadcastStateTransitionFactory = require('./broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContractFactory');
const getDocumentsFactory = require('./getDocumentsFactory');
const getIdentityFactory = require('./getIdentityFactory');
const getIdentityIdsByPublicKeyHashesFactory = require('./getIdentityIdsByPublicKeyHashesFactory');
const getIdentitiesByPublicKeyHashesFactory = require('./getIdentitiesByPublicKeyHashesFactory');
const waitForStateTransitionResultFactory = require('./waitForStateTransitionResultFactory');

class PlatformMethodsFacade {
  /**
   * @param {GrpcTransport} grpcTransport
   */
  constructor(grpcTransport) {
    this.broadcastStateTransition = broadcastStateTransitionFactory(grpcTransport);
    this.getDataContract = getDataContractFactory(grpcTransport);
    this.getDocuments = getDocumentsFactory(grpcTransport);
    this.getIdentity = getIdentityFactory(grpcTransport);
    this.getIdentityIdsByPublicKeyHashes = getIdentityIdsByPublicKeyHashesFactory(grpcTransport);
    this.getIdentitiesByPublicKeyHashes = getIdentitiesByPublicKeyHashesFactory(grpcTransport);
    this.waitForStateTransitionResult = waitForStateTransitionResultFactory(grpcTransport);
  }
}

module.exports = PlatformMethodsFacade;

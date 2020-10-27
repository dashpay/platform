const broadcastStateTransitionFactory = require('./broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContractFactory');
const getDocumentsFactory = require('./getDocumentsFactory');
const getIdentityFactory = require('./getIdentityFactory');
const getIdentityIdsByPublicKeyHashesFactory = require('./getIdentityIdsByPublicKeyHashesFactory');
const getIdentitiesByPublicKeyHashesFactory = require('./getIdentitiesByPublicKeyHashesFactory');

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
  }
}

module.exports = PlatformMethodsFacade;

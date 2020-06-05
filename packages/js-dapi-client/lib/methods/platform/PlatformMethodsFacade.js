const broadcastStateTransitionFactory = require('./broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContractFactory');
const getDocumentsFactory = require('./getDocumentsFactory');
const getIdentityByFirstPublicKeyFactory = require('./getIdentityByFirstPublicKeyFactory');
const getIdentityFactory = require('./getIdentityFactory');
const getIdentityIdByFirstPublicKeyFactory = require('./getIdentityIdByFirstPublicKeyFactory');

class PlatformMethodsFacade {
  /**
   * @param {GrpcTransport} grpcTransport
   */
  constructor(grpcTransport) {
    this.broadcastStateTransition = broadcastStateTransitionFactory(grpcTransport);
    this.getDataContract = getDataContractFactory(grpcTransport);
    this.getDocuments = getDocumentsFactory(grpcTransport);
    this.getIdentityByFirstPublicKey = getIdentityByFirstPublicKeyFactory(grpcTransport);
    this.getIdentity = getIdentityFactory(grpcTransport);
    this.getIdentityIdByFirstPublicKey = getIdentityIdByFirstPublicKeyFactory(grpcTransport);
  }
}

module.exports = PlatformMethodsFacade;

const broadcastStateTransitionFactory = require('./broadcastStateTransition/broadcastStateTransitionFactory');
const getDataContractFactory = require('./getDataContract/getDataContractFactory');
const getDocumentsFactory = require('./getDocuments/getDocumentsFactory');
const getIdentityFactory = require('./getIdentity/getIdentityFactory');
const getIdentityIdsByPublicKeyHashesFactory = require('./getIdentityIdsByPublicKeyHashes/getIdentityIdsByPublicKeyHashesFactory');
const getIdentitiesByPublicKeyHashesFactory = require('./getIdentitiesByPublicKeyHashes/getIdentitiesByPublicKeyHashesFactory');
const waitForStateTransitionResultFactory = require('./waitForStateTransitionResult/waitForStateTransitionResultFactory');

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

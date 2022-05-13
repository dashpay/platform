const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {LRUCache} dataContractCache
 *
 * @return {registerSystemDataContract}
 */
function registerSystemDataContractFactory(
  dpp,
  identityRepository,
  dataContractRepository,
  publicKeyToIdentityIdRepository,
  blockExecutionContext,
  dataContractCache,
) {
  /**
   * @typedef registerSystemDataContract
   *
   * @param {Identifier} ownerId
   * @param {Identifier} contractId
   * @param {PublicKey} masterPublicKey
   * @param {PublicKey} secondPublicKey
   * @param {Object} documentDefinitions
   *
   * @returns {Promise<DataContract>}
   */
  async function registerSystemDataContract(
    ownerId,
    contractId,
    masterPublicKey,
    secondPublicKey,
    documentDefinitions,
  ) {
    const ownerIdentity = dpp.identity.create(
      {
        createIdentifier: () => ownerId,
      },
      [{
        key: masterPublicKey,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      }, {
        key: secondPublicKey,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      }],
    );

    await identityRepository.store(ownerIdentity, true);

    await publicKeyToIdentityIdRepository.store(masterPublicKey.hash, ownerId, true);
    await publicKeyToIdentityIdRepository.store(secondPublicKey.hash, ownerId, true);

    const dataContract = dpp.dataContract.create(
      ownerIdentity.getId(),
      documentDefinitions,
    );

    dataContract.id = contractId;

    await dataContractRepository.store(dataContract, true);

    // Store data contract in the cache
    dataContractCache.set(dataContract.getId().toString(), dataContract);

    return dataContract;
  }

  return registerSystemDataContract;
}

module.exports = registerSystemDataContractFactory;

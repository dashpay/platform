const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {RootTree} rootTree
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {IdentityStoreRepository} previousIdentityRepository
 * @param {DataContractStoreRepository} previousDataContractRepository
 * @param {PublicKeyToIdentityIdStoreRepository} previousPublicKeyToIdentityIdRepository
 * @param {RootTree} previousRootTree
 * @param {LRUCache} dataContractCache
 *
 * @return {registerSystemDataContract}
 */
function registerSystemDataContractFactory(
  dpp,
  identityRepository,
  dataContractRepository,
  publicKeyToIdentityIdRepository,
  rootTree,
  blockExecutionContext,
  previousIdentityRepository,
  previousDataContractRepository,
  previousPublicKeyToIdentityIdRepository,
  previousRootTree,
  dataContractCache,
) {
  /**
   * @typedef registerSystemDataContract
   *
   * @param {Identifier} ownerId
   * @param {Identifier} contractId
   * @param {PublicKey} publicKey
   * @param {Object} documentDefinitions
   *
   * @returns {Promise<DataContract>}
   */
  async function registerSystemDataContract(
    ownerId,
    contractId,
    publicKey,
    documentDefinitions,
  ) {
    const ownerIdentity = dpp.identity.create(
      {
        createIdentifier: () => ownerId,
      },
      [{
        key: publicKey,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      }],
    );

    await identityRepository.store(ownerIdentity);
    await previousIdentityRepository.store(ownerIdentity);

    await publicKeyToIdentityIdRepository.store(publicKey.hash, ownerId);
    await previousPublicKeyToIdentityIdRepository.store(publicKey.hash, ownerId);

    const dataContract = dpp.dataContract.create(
      ownerIdentity.getId(),
      documentDefinitions,
    );

    dataContract.id = contractId;

    await dataContractRepository.store(dataContract);
    await previousDataContractRepository.store(dataContract);

    // Store data contract in the cache
    dataContractCache.set(dataContract.getId().toString(), dataContract);

    // Rebuild root tree to accommodate for changes
    // since we're inserting data directly
    rootTree.rebuild();
    previousRootTree.rebuild();

    return dataContract;
  }

  return registerSystemDataContract;
}

module.exports = registerSystemDataContractFactory;

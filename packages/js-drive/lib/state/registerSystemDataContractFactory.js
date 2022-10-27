const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const DataContractCacheItem = require('../dataContract/DataContractCacheItem');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {PublicKeyToIdentitiesStoreRepository} publicKeyToIdentitiesRepository
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {LRUCache} dataContractCache
 *
 * @return {registerSystemDataContract}
 */
function registerSystemDataContractFactory(
  dpp,
  identityRepository,
  dataContractRepository,
  publicKeyToIdentitiesRepository,
  latestBlockExecutionContext,
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

    await identityRepository.create(ownerIdentity, {
      useTransaction: true,
    });

    await publicKeyToIdentitiesRepository.store(masterPublicKey.hash, ownerId, {
      useTransaction: true,
    });

    const dataContract = dpp.dataContract.create(
      ownerIdentity.getId(),
      documentDefinitions,
    );

    dataContract.id = contractId;

    await dataContractRepository.store(dataContract, {
      useTransaction: true,
    });

    // Store data contract in the cache
    const cacheItem = new DataContractCacheItem(dataContract, [
      new ReadOperation(dataContract.toBuffer().length),
    ]);

    dataContractCache.set(cacheItem.getKey(), cacheItem);

    return dataContract;
  }

  return registerSystemDataContract;
}

module.exports = registerSystemDataContractFactory;

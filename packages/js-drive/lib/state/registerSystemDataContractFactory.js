const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {IdentityPublicKeyStoreRepository} identityPublicKeyRepository
 * @param {LRUCache} dataContractCache
 *
 * @return {registerSystemDataContract}
 */
function registerSystemDataContractFactory(
  dpp,
  identityRepository,
  dataContractRepository,
  identityPublicKeyRepository,
) {
  /**
   * @typedef registerSystemDataContract
   *
   * @param {Identifier} ownerId
   * @param {Identifier} contractId
   * @param {PublicKey} masterPublicKey
   * @param {PublicKey} secondPublicKey
   * @param {Object} documentDefinitions
   * @param {BlockInfo} blockInfo
   *
   * @returns {Promise<DataContract>}
   */
  async function registerSystemDataContract(
    ownerId,
    contractId,
    masterPublicKey,
    secondPublicKey,
    documentDefinitions,
    blockInfo,
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

    await identityRepository.create(ownerIdentity, blockInfo, {
      useTransaction: true,
    });

    const dataContract = dpp.dataContract.create(
      ownerIdentity.getId(),
      documentDefinitions,
    );

    dataContract.id = contractId;

    await dataContractRepository.create(dataContract, blockInfo, {
      useTransaction: true,
    });

    return dataContract;
  }

  return registerSystemDataContract;
}

module.exports = registerSystemDataContractFactory;

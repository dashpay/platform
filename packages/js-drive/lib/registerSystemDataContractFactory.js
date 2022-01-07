const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const { PublicKey } = require('@dashevo/dashcore-lib');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {RootTree} rootTree
 *
 * @return {registerSystemDataContract}
 */
function registerSystemDataContractFactory(
  dpp,
  identityRepository,
  dataContractRepository,
  rootTree,
) {
  /**
   * @typedef registerSystemDataContract
   *
   * @param {string} ownerIdString
   * @param {string} contractIdString
   * @param {string} publicKeyString
   * @param {Object} documentDefinitions
   *
   * @returns {Promise<void>}
   */
  async function registerSystemDataContract(
    ownerIdString,
    contractIdString,
    publicKeyString,
    documentDefinitions,
  ) {
    const ownerIdentity = dpp.identity.create(
      {
        createIdentifier: () => Identifier.from(
          Buffer.from(ownerIdString, 'hex'),
        ),
      },
      [{
        key: PublicKey.fromString(
          publicKeyString,
        ),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      }],
    );

    await identityRepository.store(ownerIdentity);

    const dataContract = dpp.dataContract.create(
      ownerIdentity.getId(),
      documentDefinitions,
    );

    dataContract.id = Identifier.from(
      Buffer.from(contractIdString, 'hex'),
    );

    await dataContractRepository.store(dataContract);

    // Rebuild root tree to accomodate for changes
    // since we're inserting data directly
    rootTree.rebuild();
  }

  return registerSystemDataContract;
}

module.exports = registerSystemDataContractFactory;

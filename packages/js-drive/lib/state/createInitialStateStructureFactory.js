const SpentAssetLockTransactionsRepository = require('../identity/SpentAssetLockTransactionsRepository');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {GroveDBStore} groveDBStore
 * @return {createInitialStateStructure}
 */
function createInitialStateStructureFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
  spentAssetLockTransactionsRepository,
  dataContractRepository,
  groveDBStore,
) {
  /**
   * @typedef {createInitialStateStructure}
   * @return {Promise<Array>}
   */
  async function createInitialStateStructure() {
    await identityRepository.createTree({ useTransaction: true });

    await publicKeyToIdentityIdRepository.createTree({ useTransaction: true });

    await dataContractRepository.createTree({ useTransaction: true });

    // Create Misc tree
    await groveDBStore.createTree(
      [],
      SpentAssetLockTransactionsRepository.TREE_PATH[0],
      { useTransaction: true },
    );

    // Add spent asset lock tree
    await spentAssetLockTransactionsRepository.createTree({ useTransaction: true });
  }

  return createInitialStateStructure;
}

module.exports = createInitialStateStructureFactory;

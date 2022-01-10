const DataContractStoreRepository = require('../dataContract/DataContractStoreRepository');
const IdentityStoreRepository = require('../identity/IdentityStoreRepository');
const PublicKeyToIdentityIdStoreRepository = require('../identity/PublicKeyToIdentityIdStoreRepository');

/**
 * @param {GroveDBStore} groveDBStore
 *
 * @return {createInitialStateStructure}
 */

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @return {createInitialStateStructure}
 */
function createInitialStateStructureFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
  spentAssetLockTransactionsRepository,
  dataContractRepository,
) {
  /**
   * @typedef {createInitialStateStructure}
   * @return {Promise<Array>}
   */
  async function createInitialStateStructure() {
    return Promise.all([
      identityRepository.createTree(),
      publicKeyToIdentityIdRepository.createTree(),
      spentAssetLockTransactionsRepository.createTree(),
      dataContractRepository.createTree(),
    ]);
  }

  return createInitialStateStructure;
}

module.exports = createInitialStateStructureFactory;

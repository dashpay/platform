const cbor = require('cbor');
const CreditsDistributionPool = require('./CreditsDistributionPool');

class CreditsDistributionPoolCommonStoreRepository {
  /**
   *
   * @param {CommonStore} commonStore
   */
  constructor(commonStore) {
    this.storage = commonStore;
  }

  /**
   * Store Credits Distribution Pool
   *
   * @param {CreditsDistributionPool} creditsDistributionPool
   * @param {MerkDbTransaction} transaction
   * @return {this}
   */
  async store(creditsDistributionPool, transaction = undefined) {
    await this.storage.put(
      CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
      cbor.encodeCanonical(creditsDistributionPool.toJSON()),
      transaction,
    );

    return this;
  }

  /**
   * Fetch Credits Distribution Pool
   *
   * @param {MerkDbTransaction} transaction
   * @return {CreditsDistributionPool}
   */
  async fetch(transaction = undefined) {
    const creditsDistributionPoolEncoded = await this.storage.get(
      CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
      transaction,
    );

    if (!creditsDistributionPoolEncoded) {
      return new CreditsDistributionPool();
    }

    const { amount } = cbor.decode(creditsDistributionPoolEncoded);

    return new CreditsDistributionPool(amount);
  }
}

CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME = Buffer.from('CreditsDistributionPool');

module.exports = CreditsDistributionPoolCommonStoreRepository;

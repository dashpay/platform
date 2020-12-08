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
   * Store chain info
   *
   * @param {CreditsDistributionPool} creditsDistributionPool
   * @return {this}
   */
  async store(creditsDistributionPool) {
    await this.storage.put(
      CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
      cbor.encodeCanonical(creditsDistributionPool.toJSON()),
    );

    return this;
  }

  /**
   * Fetch chain info
   *
   * @return {CreditsDistributionPool}
   */
  async fetch() {
    const creditsDistributionPoolEncoded = await this.storage.get(
      CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
    );

    if (!creditsDistributionPoolEncoded) {
      return new CreditsDistributionPool();
    }

    const {
      amount,
    } = cbor.decode(creditsDistributionPoolEncoded);

    return new CreditsDistributionPool(
      amount,
    );
  }
}

CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME = Buffer.from('CreditsDistributionPool');

module.exports = CreditsDistributionPoolCommonStoreRepository;

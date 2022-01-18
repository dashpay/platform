const cbor = require('cbor');
const CreditsDistributionPool = require('./CreditsDistributionPool');

class CreditsDistributionPoolRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store Credits Distribution Pool
   *
   * @param {CreditsDistributionPool} creditsDistributionPool
   * @param {boolean} [useTransaction=false]
   * @return {this}
   */
  async store(creditsDistributionPool, useTransaction = false) {
    await this.storage.putAux(
      CreditsDistributionPoolRepository.COMMON_STORE_KEY_NAME,
      cbor.encodeCanonical(creditsDistributionPool.toJSON()),
      { useTransaction },
    );

    return this;
  }

  /**
   * Fetch Credits Distribution Pool
   *
   * @param {boolean} [useTransaction=false]
   * @return {CreditsDistributionPool}
   */
  async fetch(useTransaction = false) {
    const creditsDistributionPoolEncoded = await this.storage.getAux(
      CreditsDistributionPoolRepository.COMMON_STORE_KEY_NAME,
      { useTransaction },
    );

    if (!creditsDistributionPoolEncoded) {
      return new CreditsDistributionPool();
    }

    const { amount } = cbor.decode(creditsDistributionPoolEncoded);

    return new CreditsDistributionPool(amount);
  }
}

CreditsDistributionPoolRepository.COMMON_STORE_KEY_NAME = Buffer.from('CreditsDistributionPool');

module.exports = CreditsDistributionPoolRepository;

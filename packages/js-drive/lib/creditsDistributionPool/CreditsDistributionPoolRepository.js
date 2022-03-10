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
    const encodedCreditsDistributionPool = cbor.encodeCanonical(
      creditsDistributionPool.toJSON(),
    );

    await this.storage.put(
      CreditsDistributionPoolRepository.PATH,
      CreditsDistributionPoolRepository.KEY,
      encodedCreditsDistributionPool,
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
    const creditsDistributionPoolEncoded = await this.storage.get(
      CreditsDistributionPoolRepository.PATH,
      CreditsDistributionPoolRepository.KEY,
      { useTransaction },
    );

    if (!creditsDistributionPoolEncoded) {
      return new CreditsDistributionPool();
    }

    const { amount } = cbor.decode(creditsDistributionPoolEncoded);

    return new CreditsDistributionPool(amount);
  }
}

CreditsDistributionPoolRepository.PATH = [Buffer.from([3])];
CreditsDistributionPoolRepository.KEY = Buffer.from([1]);

module.exports = CreditsDistributionPoolRepository;

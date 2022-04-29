const cbor = require('cbor');

const CreditsDistributionPool = require('./CreditsDistributionPool');
const StorageResult = require('../storage/StorageResult');

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
   * @return {Promise<StorageResult<void>>}
   */
  async store(creditsDistributionPool, useTransaction = false) {
    const encodedCreditsDistributionPool = cbor.encodeCanonical(
      creditsDistributionPool.toJSON(),
    );

    const result = await this.storage.put(
      CreditsDistributionPoolRepository.PATH,
      CreditsDistributionPoolRepository.KEY,
      encodedCreditsDistributionPool,
      { useTransaction },
    );

    result.setValue(undefined);

    return result;
  }

  /**
   * Fetch Credits Distribution Pool
   *
   * @param {boolean} [useTransaction=false]
   * @return {Promise<StorageResult<CreditsDistributionPool>>}
   */
  async fetch(useTransaction = false) {
    const result = await this.storage.get(
      CreditsDistributionPoolRepository.PATH,
      CreditsDistributionPoolRepository.KEY,
      { useTransaction },
    );

    if (result.isEmpty()) {
      return new StorageResult(
        new CreditsDistributionPool(),
        result.getOperations(),
      );
    }

    const { amount } = cbor.decode(result.getValue());

    return new StorageResult(
      new CreditsDistributionPool(amount),
      result.getOperations(),
    );
  }
}

CreditsDistributionPoolRepository.PATH = [Buffer.from([3])];
CreditsDistributionPoolRepository.KEY = Buffer.from([1]);

module.exports = CreditsDistributionPoolRepository;

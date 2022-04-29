const cbor = require('cbor');

const CreditsDistributionPool = require('./CreditsDistributionPool');
const RepositoryResult = require('../storage/RepositoryResult');

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
   * @return {Promise<RepositoryResult<void>>}
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

    return new RepositoryResult(
      undefined,
      result.getOperations(),
    );
  }

  /**
   * Fetch Credits Distribution Pool
   *
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<CreditsDistributionPool>>}
   */
  async fetch(useTransaction = false) {
    const creditsDistributionPoolEncodedResult = await this.storage.get(
      CreditsDistributionPoolRepository.PATH,
      CreditsDistributionPoolRepository.KEY,
      { useTransaction },
    );

    if (!creditsDistributionPoolEncodedResult.getResult()) {
      return new RepositoryResult(
        new CreditsDistributionPool(),
        creditsDistributionPoolEncodedResult.getOperations(),
      );
    }

    const { amount } = cbor.decode(creditsDistributionPoolEncodedResult.getResult());

    return new RepositoryResult(
      new CreditsDistributionPool(amount),
      creditsDistributionPoolEncodedResult.getOperations(),
    );
  }
}

CreditsDistributionPoolRepository.PATH = [Buffer.from([3])];
CreditsDistributionPoolRepository.KEY = Buffer.from([1]);

module.exports = CreditsDistributionPoolRepository;

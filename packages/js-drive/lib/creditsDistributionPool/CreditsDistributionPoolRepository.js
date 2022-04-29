const cbor = require('cbor');

const Write = require('@dashevo/dpp/lib/stateTransition/fees/operations/WriteOperation');
const Read = require('@dashevo/dpp/lib/stateTransition/fees/operations/ReadOperation');

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

    return {
      result: this,
      operations: [
        new Write(
          CreditsDistributionPoolRepository.KEY.length,
          encodedCreditsDistributionPool.length,
        ),
      ],
    };
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
      return {
        result: new CreditsDistributionPool(),
        operations: [
          new Read(
            CreditsDistributionPoolRepository.KEY.length,
            CreditsDistributionPoolRepository.PATH.reduce(
              (size, pathItem) => size + pathItem.length, 0,
            ).length,
            0,
          ),
        ],
      };
    }

    const { amount } = cbor.decode(creditsDistributionPoolEncoded);

    return {
      result: new CreditsDistributionPool(amount),
      operations: [
        new Read(
          CreditsDistributionPoolRepository.KEY.length,
          CreditsDistributionPoolRepository.PATH.reduce(
            (size, pathItem) => size + pathItem.length, 0,
          ).length,
          creditsDistributionPoolEncoded.length,
        ),
      ],
    };
  }
}

CreditsDistributionPoolRepository.PATH = [Buffer.from([3])];
CreditsDistributionPoolRepository.KEY = Buffer.from([1]);

module.exports = CreditsDistributionPoolRepository;

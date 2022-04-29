const Write = require('@dashevo/dpp/lib/stateTransition/fees/operations/WriteOperation');
const Read = require('@dashevo/dpp/lib/stateTransition/fees/operations/ReadOperation');

class SpentAssetLockTransactionsRepository {
  /**
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {boolean} [useTransaction=false]
   *
   * @return {SpentAssetLockTransactionsRepository}
   */
  async store(outPointBuffer, useTransaction = false) {
    const emptyValue = Buffer.from([0]);

    await this.storage.put(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      emptyValue,
      { useTransaction },
    );

    return {
      result: this,
      operations: [
        new Write(
          outPointBuffer.length,
          emptyValue.length,
        ),
      ],
    };
  }

  /**
   * Fetch the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {boolean} [useTransaction=false]
   *
   * @return {null|Buffer}
   */
  async fetch(outPointBuffer, useTransaction = false) {
    const result = await this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      { useTransaction },
    );

    return {
      result,
      operations: [
        new Read(
          outPointBuffer.length,
          SpentAssetLockTransactionsRepository.TREE_PATH.reduce(
            (size, pathItem) => size + pathItem.length, 0,
          ),
          result.length,
        ),
      ],
    };
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<SpentAssetLockTransactionsRepository>}
   */
  async createTree(options = {}) {
    const rootTreePath = [SpentAssetLockTransactionsRepository.TREE_PATH[0]];
    const treePath = SpentAssetLockTransactionsRepository.TREE_PATH[1];

    await this.storage.createTree(
      rootTreePath,
      treePath,
      options,
    );

    return {
      result: this,
      operations: [
        new Write(
          treePath.length,
          32,
        ),
      ],
    };
  }
}

SpentAssetLockTransactionsRepository.TREE_PATH = [
  Buffer.from([3]),
  Buffer.from([0]),
];

module.exports = SpentAssetLockTransactionsRepository;

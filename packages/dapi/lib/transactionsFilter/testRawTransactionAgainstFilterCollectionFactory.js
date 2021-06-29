const { Transaction } = require('@dashevo/dashcore-lib');

/**
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @return {testRawTransactionAgainstFilterCollection}
 */
function testRawTransactionAgainstFilterCollectionFactory(bloomFilterEmitterCollection) {
  /**
   * Test a raw transaction against bloom filter collection
   *
   * @typedef testRawTransactionAgainstFilterCollection
   * @param {Buffer} rawTransaction
   */
  function testRawTransactionAgainstFilterCollection(rawTransaction) {
    const transaction = new Transaction(rawTransaction);

    bloomFilterEmitterCollection.test(transaction);
  }

  return testRawTransactionAgainstFilterCollection;
}

module.exports = testRawTransactionAgainstFilterCollectionFactory;

const {
  BloomFilter, Address,
} = require('@dashevo/dashcore-lib');
const { BLOOM_FALSE_POSITIVE_RATE } = require('../../../CONSTANTS');

/**
 * @param {string[]} addresses
 * @returns {BloomFilter}
 */
const createBloomFilter = (addresses) => {
  const bloomFilter = BloomFilter.create(addresses.length, BLOOM_FALSE_POSITIVE_RATE);
  addresses.forEach((addressString) => {
    const address = new Address(addressString);
    bloomFilter.insert(address);
  });

  return bloomFilter;
};

/**
 * Filters transactions for specific addresses list
 * @param {Transaction[]} transactions
 * @param {string[]} addresses
 * @param {string} network
 * @returns {Transaction[]}
 */
const filterTransactionsForAddresses = (transactions, addresses, network = 'livenet') => {
  const filteredTransactions = transactions.filter((tx) => {
    const txPayload = [...tx.inputs, ...tx.outputs];
    for (let i = 0; i < txPayload.length; i += 1) {
      const payloadItem = txPayload[i];

      if (payloadItem.script) {
        const address = payloadItem.script.toAddress(network).toString();

        if (addresses.includes(address)) {
          return true;
        }
      }
    }

    return false;
  });

  return filteredTransactions;
};

module.exports = {
  createBloomFilter,
  filterTransactionsForAddresses,
};

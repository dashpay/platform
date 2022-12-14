const {
  BloomFilter, Address, MerkleBlock, Transaction, InstantLock,
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
    bloomFilter.insert(address.hashBuffer);
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
        const address = payloadItem.script.toAddress(network);
        if (address && addresses.includes(address.toString())) {
          return true;
        }
      }
    }

    return false;
  });

  return filteredTransactions;
};

/**
 * @private
 * @param {proto.org.dash.platform.dapi.v0.RawTransactions} rawTransactions
 * @param {string[]} addresses
 * @param {string} network
 * @returns {Transaction[]} transactions
 */
const parseRawTransactions = (rawTransactions, addresses, network) => {
  const transactions = rawTransactions.getTransactionsList()
    .map((rawTransaction) => new Transaction(Buffer.from(rawTransaction)));

  return filterTransactionsForAddresses(
    transactions,
    addresses,
    network,
  );
};

/**
 * @private
 * @param {TypedArray} rawMerkleBlock
 * @returns {MerkleBlock}
 */
const parseRawMerkleBlock = (rawMerkleBlock) => new MerkleBlock(Buffer.from(rawMerkleBlock));

/**
 * @param {KeyChainStore} keyChainStore
 * @returns {string[]}
 */
const getAddressesToSync = (keyChainStore) => keyChainStore.getKeyChains()
  .map((keychain) => keychain.getWatchedAddresses())
  .reduce((pre, cur) => pre.concat(cur));

/**
 * @private
 * @param {proto.org.dash.platform.dapi.v0.InstantSendLockMessages} rawInstantLocks
 * @returns {InstantLock[]} instantLocks
 */
const parseRawInstantLocks = (rawInstantLocks) => rawInstantLocks
  .getMessagesList()
  .map((instantSendLock) => new InstantLock(Buffer.from(instantSendLock)));

/**
 * @param merkleBlock
 * @returns {Set<string>}
 */
const getTxHashesFromMerkleBlock = (merkleBlock) => merkleBlock
  .hashes.reduce((set, hashHex) => {
    const hash = Buffer.from(hashHex, 'hex').reverse();
    set.add(hash.toString('hex'));
    return set;
  }, new Set());

module.exports = {
  createBloomFilter,
  filterTransactionsForAddresses,
  parseRawTransactions,
  parseRawMerkleBlock,
  parseRawInstantLocks,
  getAddressesToSync,
  getTxHashesFromMerkleBlock,
};

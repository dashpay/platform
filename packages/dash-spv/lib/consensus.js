const { hasValidTarget } = require('@dashevo/dark-gravity-wave');
const merkleProofs = require('./merkleproofs');
const utils = require('./utils');

const MIN_TIMESTAMP_HEADERS = 11;
const MIN_DGW_HEADERS = 24;

function getMedianTimestamp(headers) {
  const timestamps = headers.map(h => h.time);
  const median = (arr) => {
    const mid = Math.floor(arr.length / 2);
    const nums = [...arr].sort((a, b) => a - b);
    return arr.length % 2 !== 0 ? nums[mid] : (nums[mid - 1] + nums[mid]) / 2;
  };
  return median(timestamps);
}

// Must be strictly greater than the median time of the previous 11 blocks.
// https://dash-docs.github.io/en/developer-reference#block-headers
function hasGreaterThanMedianTimestamp(newHeader, previousHeaders) {
  if (previousHeaders.length < MIN_TIMESTAMP_HEADERS) return true;
  const headerNormalised = utils.normalizeHeader(newHeader);
  const normalizedLatestHeaders = previousHeaders.slice(
    Math.max(previousHeaders.length - MIN_TIMESTAMP_HEADERS, 0),
  ).map(h => utils.normalizeHeader(h));
  return getMedianTimestamp(normalizedLatestHeaders) < headerNormalised.time;
}

function isValidBlockHeader(newHeader, previousHeaders, network = 'mainnet') {
  if (previousHeaders.length > MIN_DGW_HEADERS) {
    return newHeader.validProofOfWork()
      && newHeader.validTimestamp()
      && hasGreaterThanMedianTimestamp(newHeader, previousHeaders)
      && hasValidTarget(
        utils.getDgwBlock(newHeader), previousHeaders.map(h => utils.getDgwBlock(h)), network,
      );
  }
  return newHeader.validProofOfWork()
    && newHeader.validTimestamp()
    && hasGreaterThanMedianTimestamp(newHeader, previousHeaders);
}

/**
 * validates an array of tx hashes or Transaction instances
 * against a merkleblock and the local header chain
 * @param {Transaction[]|string[]} transactions
 * @param {MerkleBlock} merkleBlock - a MerkleBlock instance
 * @param {SpvChain} headerChain - an instance of an SpvChain
 * @return {boolean}
 */
async function areValidTransactions(transactions, merkleBlock, headerChain) {
  if (!Array.isArray(transactions) || transactions.length <= 0) {
    throw new Error('Please check that transactions parameter is a non-empty array');
  }
  const localHeader = await headerChain.getHeader(merkleBlock.header.hash);
  if (!localHeader) {
    return false;
  }
  return merkleProofs.validateTxProofs(merkleBlock, transactions);
}

module.exports = {
  isValidBlockHeader,
  areValidTransactions,
};

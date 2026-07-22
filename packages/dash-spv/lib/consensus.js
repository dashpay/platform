const { hasValidTarget } = require('@dashevo/dark-gravity-wave');
const DashUtil = require('@dashevo/dash-util');
const merkleProofs = require('./merkleproofs');
const utils = require('./utils');

const MIN_TIMESTAMP_HEADERS = 11;
const MIN_DGW_HEADERS = 24;

const NETWORK_POW_LIMITS = {
  mainnet: 0x1e0ffff0,
  testnet: 0x1e0fffff,
  devnet: 0x207fffff,
  regtest: 0x207fffff,
};

function getMedianTimestamp(headers) {
  const timestamps = headers.map((h) => h.time).sort((a, b) => a - b);
  return timestamps[Math.floor(timestamps.length / 2)];
}

// Must be strictly greater than the median time of the previous 11 blocks.
// https://dash-docs.github.io/en/developer-reference#block-headers
function hasGreaterThanMedianTimestamp(newHeader, previousHeaders) {
  if (previousHeaders.length < MIN_TIMESTAMP_HEADERS) {
    return true;
  }
  const normalizedHeader = utils.normalizeHeader(newHeader);
  const latestHeaders = previousHeaders
    .slice(-MIN_TIMESTAMP_HEADERS)
    .map((header) => utils.normalizeHeader(header));
  return getMedianTimestamp(latestHeaders) < normalizedHeader.time;
}

function hasCanonicalTarget(header, network) {
  const powLimit = NETWORK_POW_LIMITS[network];
  if (!powLimit || !Number.isInteger(header.bits)) {
    return false;
  }

  try {
    const target = DashUtil.expandTarget(header.bits);
    const canonicalBits = DashUtil.compressTarget(target);
    const maximumTarget = DashUtil.expandTarget(powLimit);

    return canonicalBits === header.bits && target.compare(maximumTarget) <= 0;
  } catch {
    return false;
  }
}

function isValidBlockHeaderWithoutContext(newHeader, network = 'mainnet') {
  const normalizedHeader = utils.normalizeHeader(newHeader);

  return hasCanonicalTarget(normalizedHeader, network)
    && utils.validProofOfWork(normalizedHeader)
    && normalizedHeader.validTimestamp();
}

function isValidBlockHeader(newHeader, previousHeaders, network = 'mainnet') {
  const normalizedHeader = utils.normalizeHeader(newHeader);
  const normalizedPreviousHeaders = previousHeaders.map((header) => utils.normalizeHeader(header));

  if (!isValidBlockHeaderWithoutContext(normalizedHeader, network)
    || !hasGreaterThanMedianTimestamp(normalizedHeader, normalizedPreviousHeaders)) {
    return false;
  }

  // A trusted checkpoint may contain fewer than a full DGW window. Proof of
  // work, the network pow limit, and timestamp rules still apply immediately;
  // exact difficulty validation begins as soon as the required history exists.
  if (normalizedPreviousHeaders.length < MIN_DGW_HEADERS) {
    return true;
  }

  return hasValidTarget(
    utils.getDgwBlock(normalizedHeader),
    normalizedPreviousHeaders.map((header) => utils.getDgwBlock(header)),
    network,
  );
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
  isValidBlockHeaderWithoutContext,
  areValidTransactions,
};

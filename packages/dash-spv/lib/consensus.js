const merkleProofs = require('./merkleproofs');
const pow = require('./pow');
const utils = require('./utils');

function isValidBlockHeaderWithoutContext(newHeader, network = 'mainnet') {
  const normalizedHeader = utils.normalizeHeader(newHeader);

  // BlockHeader memoizes its hash, which a changed field doesn't refresh:
  // check the hash of the header as it is now, and that it matches
  const hash = utils.getHeaderHash(normalizedHeader);
  return normalizedHeader.hash === hash
    && pow.checkProofOfWork(hash, normalizedHeader.bits, network)
    && normalizedHeader.validTimestamp();
}

/**
 * Validates a header against the headers before it, as Dash Core's
 * ContextualCheckBlockHeader does for proof of work and time.
 *
 * Rules that need headers from before a trusted checkpoint the chain started
 * at are skipped; proof of work and the network limit still apply.
 *
 * @param {BlockHeader} newHeader
 * @param {BlockHeader[]} previousHeaders - oldest first, ending with the parent
 *   of newHeader. Only the last pow.MAX_HISTORY are read.
 * @param {string} network
 * @param {number} height - height of newHeader
 * @param {number} [rootHeight] - height of the first header the chain has,
 *   by default the first of previousHeaders
 * @return {boolean}
 */
function isValidBlockHeader(newHeader, previousHeaders, network, height, rootHeight) {
  if (!Number.isInteger(height) || height < 1) {
    throw new Error(`Invalid header height ${height}`);
  }

  const normalizedHeader = utils.normalizeHeader(newHeader);

  if (!isValidBlockHeaderWithoutContext(normalizedHeader, network)) {
    return false;
  }

  // Must be strictly greater than the median time of the previous 11 blocks
  const medianTimePast = pow.getMedianTimePast(previousHeaders, height);
  if (medianTimePast !== null && normalizedHeader.time <= medianTimePast) {
    return false;
  }

  return pow.hasValidDifficulty(
    normalizedHeader,
    previousHeaders,
    height,
    network,
    rootHeight,
  ) !== false;
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

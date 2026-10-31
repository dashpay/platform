/* eslint-disable no-bitwise */
const utils = require('./utils');

// Difficulty rules ported from Dash Core: src/pow.cpp, the bad-diffbits check
// in ContextualCheckBlockHeader (src/validation.cpp), and the per-network
// consensus parameters in src/chainparams.cpp. Targets are BigInts that wrap
// at 256 bits, like Core's arith_uint256.

const UINT256_MASK = (1n << 256n) - 1n;

const TARGET_SPACING = 150; // nPowTargetSpacing: 2.5 minutes
const TARGET_TIMESPAN = 24 * 60 * 60; // nPowTargetTimespan: 1 day

const DGW_PAST_BLOCKS = 24;
const MEDIAN_TIME_SPAN = 11;

/**
 * How many headers before a new one difficulty validation may read
 * (KimotoGravityWell looks back the furthest)
 */
const MAX_HISTORY = Math.floor((TARGET_TIMESPAN * 7) / TARGET_SPACING);

const POW_LIMIT = 0x00000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffn; // ~uint256(0) >> 20
const POW_LIMIT_LOW_DIFFICULTY = 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffn; // ~uint256(0) >> 1

const NETWORK_PARAMS = {
  mainnet: {
    powLimit: POW_LIMIT,
    targetSpacing: TARGET_SPACING,
    minimumDifficultyBlocks: 0,
    kgwHeight: 15200,
    dgwHeight: 34140,
    allowMinDifficultyBlocks: false,
    noRetargeting: false,
    // Blocks up to this height only need bits within 50% of the expected
    // difficulty ("architecture issues with DGW v1 and v2")
    laxDifficultyMaxHeight: 68589,
  },
  testnet: {
    powLimit: POW_LIMIT,
    targetSpacing: TARGET_SPACING,
    minimumDifficultyBlocks: 0,
    kgwHeight: 4002,
    dgwHeight: 4002,
    allowMinDifficultyBlocks: true,
    noRetargeting: false,
  },
  devnet: {
    powLimit: POW_LIMIT_LOW_DIFFICULTY,
    // Core's defaults; a devnet started with -powtargetspacing or
    // -minimumdifficultyblocks must pass the same values to SpvChain
    targetSpacing: TARGET_SPACING,
    minimumDifficultyBlocks: 0,
    kgwHeight: 4001,
    dgwHeight: 4001,
    allowMinDifficultyBlocks: true,
    noRetargeting: false,
  },
  regtest: {
    powLimit: POW_LIMIT_LOW_DIFFICULTY,
    targetSpacing: TARGET_SPACING,
    minimumDifficultyBlocks: 0,
    kgwHeight: 15200,
    dgwHeight: 34140,
    allowMinDifficultyBlocks: true,
    noRetargeting: true,
  },
};

/**
 * @param {string|{ network: string, powTargetSpacing?: number, minimumDifficultyBlocks?: number }} network -
 *   a network name, or a devnet with the settings it was started with
 * @return {object}
 */
function getNetworkParams(network) {
  const name = typeof network === 'string' ? network : network && network.network;
  const params = NETWORK_PARAMS[name];
  if (!params) {
    throw new Error(`Unsupported network ${name}`);
  }
  if (typeof network === 'string') {
    return params;
  }

  const { powTargetSpacing = params.targetSpacing, minimumDifficultyBlocks = params.minimumDifficultyBlocks } = network;
  if ((powTargetSpacing !== params.targetSpacing || minimumDifficultyBlocks !== params.minimumDifficultyBlocks)
    && name !== 'devnet') {
    throw new Error('Only devnets set their own block spacing and minimum-difficulty blocks');
  }
  if (!Number.isInteger(powTargetSpacing) || powTargetSpacing < 1
    || Math.trunc(TARGET_TIMESPAN / powTargetSpacing) > MAX_HISTORY) {
    throw new Error(`Unsupported block spacing ${powTargetSpacing}`);
  }
  if (!Number.isInteger(minimumDifficultyBlocks) || minimumDifficultyBlocks < 0) {
    throw new Error(`Invalid minimum difficulty blocks ${minimumDifficultyBlocks}`);
  }
  return { ...params, targetSpacing: powTargetSpacing, minimumDifficultyBlocks };
}

/**
 * arith_uint256::SetCompact
 * @param {number} compact
 * @return {{ target: bigint, negative: boolean, overflow: boolean }}
 */
function decodeCompact(compact) {
  const size = compact >>> 24;
  let word = compact & 0x007fffff;
  let target;
  if (size <= 3) {
    word >>>= 8 * (3 - size);
    target = BigInt(word);
  } else {
    target = (BigInt(word) << BigInt(8 * (size - 3))) & UINT256_MASK;
  }

  return {
    target,
    negative: word !== 0 && (compact & 0x00800000) !== 0,
    overflow: word !== 0 && (size > 34 || (word > 0xff && size > 33) || (word > 0xffff && size > 32)),
  };
}

/**
 * arith_uint256::GetCompact for a non-negative target
 * @param {bigint} target
 * @return {number}
 */
function encodeCompact(target) {
  let size = Math.ceil(target.toString(2).length / 8);
  if (target === 0n) {
    size = 0;
  }

  let compact = size <= 3
    ? Number(target << BigInt(8 * (3 - size)))
    : Number(target >> BigInt(8 * (size - 3)));

  if (compact & 0x00800000) {
    compact >>>= 8;
    size += 1;
  }

  return (compact | (size << 24)) >>> 0;
}

/**
 * CheckProofOfWork: bits must decode to a positive target within the network
 * limit, and the header hash must not exceed it
 * @param {string} hash - header hash as big-endian hex
 * @param {number} bits
 * @param {string|object} network - see getNetworkParams
 * @return {boolean}
 */
function checkProofOfWork(hash, bits, network) {
  const { powLimit } = getNetworkParams(network);
  if (!Number.isInteger(bits) || bits < 0 || bits > 0xffffffff) {
    return false;
  }

  const { target, negative, overflow } = decodeCompact(bits);
  if (negative || overflow || target === 0n || target > powLimit) {
    return false;
  }

  return BigInt(`0x${hash}`) <= target;
}

/**
 * ConvertBitsToDouble (difficulty as a double)
 * @param {number} bits
 * @return {number}
 */
function convertBitsToDouble(bits) {
  let shift = (bits >>> 24) & 0xff;
  let diff = 0x0000ffff / (bits & 0x00ffffff);
  while (shift < 29) {
    diff *= 256.0;
    shift += 1;
  }
  while (shift > 29) {
    diff /= 256.0;
    shift -= 1;
  }
  return diff;
}

class MissingHistoryError extends Error {
  constructor(height) {
    super(`Header at height ${height} is not in the validation history`);
    this.height = height;
  }
}

/**
 * The headers before a new one: `history` ends with the previous header, at
 * `lastHeight`, and earlier ones are looked up by height
 * @param {BlockHeader[]} history
 * @param {number} lastHeight
 */
function createAncestors(history, lastHeight) {
  return {
    history,
    lastHeight,
    last: history.length ? utils.normalizeHeader(history[history.length - 1]) : undefined,
  };
}

function ancestorAt({ history, lastHeight }, height) {
  const index = history.length - 1 - (lastHeight - height);
  if (index < 0 || height > lastHeight) {
    throw new MissingHistoryError(height);
  }
  return utils.normalizeHeader(history[index]);
}

function compactTarget(header) {
  return decodeCompact(header.bits).target;
}

/** DifficultyAdjustmentInterval */
function getDifficultyAdjustmentInterval(params) {
  return Math.trunc(TARGET_TIMESPAN / params.targetSpacing);
}

/** CalculateNextWorkRequired */
function calculateNextWorkRequired(last, firstBlockTime, params) {
  if (params.noRetargeting) {
    return last.bits;
  }

  let actualTimespan = last.time - firstBlockTime;
  if (actualTimespan < TARGET_TIMESPAN / 4) {
    actualTimespan = TARGET_TIMESPAN / 4;
  }
  if (actualTimespan > TARGET_TIMESPAN * 4) {
    actualTimespan = TARGET_TIMESPAN * 4;
  }

  let target = (compactTarget(last) * BigInt(actualTimespan)) & UINT256_MASK;
  target /= BigInt(TARGET_TIMESPAN);

  if (target > params.powLimit) {
    target = params.powLimit;
  }

  return encodeCompact(target);
}

/** GetNextWorkRequiredBTC */
function getNextWorkRequiredBTC(ancestors, newHeader, params) {
  const { last, lastHeight } = ancestors;
  const powLimitCompact = encodeCompact(params.powLimit);

  const interval = getDifficultyAdjustmentInterval(params);

  // Only change once per interval
  if ((lastHeight + 1) % interval !== 0) {
    if (params.allowMinDifficultyBlocks) {
      // A block more than two spacings after the previous one may be mined
      // at minimum difficulty
      if (newHeader.time > last.time + params.targetSpacing * 2) {
        return powLimitCompact;
      }

      // Return the last non-special-min-difficulty-rules block
      let height = lastHeight;
      let header = last;
      while (height > 0
        && height % interval !== 0
        && header.bits === powLimitCompact) {
        height -= 1;
        header = ancestorAt(ancestors, height);
      }
      return header.bits;
    }
    return last.bits;
  }

  // Go back by what we want to be 1 day worth of blocks
  const first = ancestorAt(ancestors, lastHeight - (interval - 1));
  return calculateNextWorkRequired(last, first.time, params);
}

/** KimotoGravityWell */
function kimotoGravityWell(ancestors, params) {
  const { last: blockLastSolved, lastHeight } = ancestors;
  const pastBlocksMin = Math.floor(Math.floor(TARGET_TIMESPAN * 0.025) / params.targetSpacing);
  const pastBlocksMax = Math.floor((TARGET_TIMESPAN * 7) / params.targetSpacing);

  if (lastHeight === 0 || lastHeight < pastBlocksMin) {
    return encodeCompact(params.powLimit);
  }

  let pastBlocksMass = 0;
  let pastRateActualSeconds = 0;
  let pastRateTargetSeconds = 0;
  let pastDifficultyAverage = 0n;
  let pastDifficultyAveragePrev = 0n;

  for (let i = 1, height = lastHeight; height > 0; i += 1, height -= 1) {
    if (i > pastBlocksMax) {
      break;
    }
    const blockReading = ancestorAt(ancestors, height);
    pastBlocksMass += 1;

    pastDifficultyAverage = compactTarget(blockReading);
    if (i > 1) {
      // handle negative arith_uint256
      if (pastDifficultyAverage >= pastDifficultyAveragePrev) {
        pastDifficultyAverage = ((pastDifficultyAverage - pastDifficultyAveragePrev) / BigInt(i))
          + pastDifficultyAveragePrev;
      } else {
        pastDifficultyAverage = pastDifficultyAveragePrev
          - ((pastDifficultyAveragePrev - pastDifficultyAverage) / BigInt(i));
      }
    }
    pastDifficultyAveragePrev = pastDifficultyAverage;

    pastRateActualSeconds = blockLastSolved.time - blockReading.time;
    pastRateTargetSeconds = params.targetSpacing * pastBlocksMass;
    let pastRateAdjustmentRatio = 1;
    if (pastRateActualSeconds < 0) {
      pastRateActualSeconds = 0;
    }
    if (pastRateActualSeconds !== 0 && pastRateTargetSeconds !== 0) {
      pastRateAdjustmentRatio = pastRateTargetSeconds / pastRateActualSeconds;
    }
    const eventHorizonDeviation = 1 + (0.7084 * ((pastBlocksMass / 28.2) ** -1.228));
    const eventHorizonDeviationFast = eventHorizonDeviation;
    const eventHorizonDeviationSlow = 1 / eventHorizonDeviation;

    if (pastBlocksMass >= pastBlocksMin
      && (pastRateAdjustmentRatio <= eventHorizonDeviationSlow
        || pastRateAdjustmentRatio >= eventHorizonDeviationFast)) {
      break;
    }
  }

  let target = pastDifficultyAverage;
  if (pastRateActualSeconds !== 0 && pastRateTargetSeconds !== 0) {
    target = (target * BigInt(pastRateActualSeconds)) & UINT256_MASK;
    target /= BigInt(pastRateTargetSeconds);
  }

  if (target > params.powLimit) {
    target = params.powLimit;
  }

  return encodeCompact(target);
}

/** DarkGravityWave */
function darkGravityWave(ancestors, params) {
  const { last, lastHeight } = ancestors;

  // make sure we have at least (DGW_PAST_BLOCKS + 1) blocks
  if (lastHeight < DGW_PAST_BLOCKS) {
    return encodeCompact(params.powLimit);
  }

  let pastTargetAverage;
  let first;
  for (let countBlocks = 1; countBlocks <= DGW_PAST_BLOCKS; countBlocks += 1) {
    first = ancestorAt(ancestors, lastHeight - countBlocks + 1);
    const target = compactTarget(first);
    if (countBlocks === 1) {
      pastTargetAverage = target;
    } else {
      // Core notes this is not really an average; the exact integer steps matter
      pastTargetAverage = ((pastTargetAverage * BigInt(countBlocks) + target) & UINT256_MASK)
        / BigInt(countBlocks + 1);
    }
  }

  let actualTimespan = last.time - first.time;
  // Spans DGW_PAST_BLOCKS - 1 intervals; Core keeps this off-by-one
  const targetTimespan = DGW_PAST_BLOCKS * params.targetSpacing;
  if (actualTimespan < Math.trunc(targetTimespan / 3)) {
    actualTimespan = Math.trunc(targetTimespan / 3);
  }
  if (actualTimespan > targetTimespan * 3) {
    actualTimespan = targetTimespan * 3;
  }

  let target = (pastTargetAverage * BigInt(actualTimespan)) & UINT256_MASK;
  target /= BigInt(targetTimespan);

  if (target > params.powLimit) {
    target = params.powLimit;
  }

  return encodeCompact(target);
}

/** GetNextWorkRequired */
function getNextWorkRequired(ancestors, newHeader, params) {
  const { last, lastHeight } = ancestors;

  // only active on devnets
  if (lastHeight < params.minimumDifficultyBlocks) {
    return encodeCompact(params.powLimit);
  }

  if (lastHeight + 1 < params.kgwHeight) {
    return getNextWorkRequiredBTC(ancestors, newHeader, params);
  }

  if (params.noRetargeting) {
    return encodeCompact(params.powLimit);
  }

  if (params.allowMinDifficultyBlocks) {
    // recent block is more than 2 hours old
    if (newHeader.time > last.time + 2 * 60 * 60) {
      return encodeCompact(params.powLimit);
    }
    // recent block is more than 10 minutes old
    if (newHeader.time > last.time + params.targetSpacing * 4) {
      const target = (compactTarget(last) * 10n) & UINT256_MASK;
      return encodeCompact(target > params.powLimit ? params.powLimit : target);
    }
  }

  if (lastHeight + 1 < params.dgwHeight) {
    return kimotoGravityWell(ancestors, params);
  }

  return darkGravityWave(ancestors, params);
}

/**
 * The bits Core requires of a new header (GetNextWorkRequired)
 *
 * @param {BlockHeader} newHeader
 * @param {BlockHeader[]} history - headers before newHeader, oldest first,
 *   ending with its parent. Only the last MAX_HISTORY are read.
 * @param {number} height - height of newHeader
 * @param {string|object} network - see getNetworkParams
 * @param {number} [rootHeight] - height of the first header the chain has,
 *   by default the first one in history
 * @return {number|null} null when the rules need headers from before the
 *   trusted checkpoint the chain starts at
 */
function getExpectedBits(newHeader, history, height, network, rootHeight = height - history.length) {
  const params = getNetworkParams(network);
  if (history.length === 0) {
    throw new Error(`No headers before height ${height} to validate its difficulty with`);
  }

  try {
    return getNextWorkRequired(createAncestors(history, height - 1), newHeader, params);
  } catch (e) {
    if (!(e instanceof MissingHistoryError)) {
      throw e;
    }
    if (e.height >= rootHeight) {
      // The chain has this header; only a too-short history leaves it out
      throw new Error(`Difficulty at height ${height} needs the header at ${e.height}, which the validation history leaves out`);
    }
    return null;
  }
}

/**
 * Checks a header's bits the way Core's ContextualCheckBlockHeader does.
 * Takes the arguments of getExpectedBits.
 *
 * @return {boolean|null} null when the rules can't be applied
 */
function hasValidDifficulty(newHeader, history, height, network, rootHeight) {
  const params = getNetworkParams(network);
  const expectedBits = getExpectedBits(newHeader, history, height, network, rootHeight);
  if (expectedBits === null) {
    return null;
  }

  if (params.laxDifficultyMaxHeight !== undefined && height <= params.laxDifficultyMaxHeight) {
    const difficulty = convertBitsToDouble(newHeader.bits);
    const expectedDifficulty = convertBitsToDouble(expectedBits);
    return Math.abs(difficulty - expectedDifficulty) <= difficulty * 0.5;
  }

  return newHeader.bits === expectedBits;
}

/**
 * Median time of the up to 11 headers before a new one (GetMedianTimePast)
 * @param {BlockHeader[]} history - ends with the parent of the new header
 * @param {number} height - height of the new header
 * @return {number|null} null when the history is too short to tell
 */
function getMedianTimePast(history, height) {
  const count = Math.min(MEDIAN_TIME_SPAN, height);
  if (history.length < count) {
    return null;
  }
  const times = history.slice(-count)
    .map((header) => utils.normalizeHeader(header).time)
    .sort((a, b) => a - b);
  return times[Math.floor(times.length / 2)];
}

module.exports = {
  MAX_HISTORY,
  getNetworkParams,
  checkProofOfWork,
  getExpectedBits,
  hasValidDifficulty,
  getMedianTimePast,
  decodeCompact,
  encodeCompact,
};

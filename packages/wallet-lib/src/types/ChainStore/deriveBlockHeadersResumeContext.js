/**
 * Derives the persisted header context that can safely resume synchronization.
 *
 * @param {ChainStoreState} state
 * @returns {{
 *   blockHeaders: Array,
 *   firstHeaderHeight: number,
 *   startBlockHeight: number,
 *   requiresHeaderStateReset: boolean,
 * }}
 */
function deriveBlockHeadersResumeContext(state) {
  const {
    blockHeaders,
    lastSyncedHeaderHeight,
    headersMetadata,
    hashesByHeight,
  } = state;

  if (!Array.isArray(blockHeaders)) {
    throw new Error('Invalid block headers: expected an array');
  }

  if (!Number.isSafeInteger(lastSyncedHeaderHeight) || lastSyncedHeaderHeight < -1) {
    throw new Error(`Invalid last synced header height ${lastSyncedHeaderHeight}`);
  }

  if (!(headersMetadata instanceof Map)) {
    throw new Error('Invalid headers metadata: expected a Map');
  }

  if (!(hashesByHeight instanceof Map)) {
    throw new Error('Invalid header hashes by height: expected a Map');
  }

  const firstHeaderHeight = lastSyncedHeaderHeight - blockHeaders.length + 1;
  const canResume = blockHeaders.length >= 2 && firstHeaderHeight >= 0;

  if (canResume) {
    return {
      blockHeaders,
      firstHeaderHeight,
      startBlockHeight: lastSyncedHeaderHeight,
      requiresHeaderStateReset: false,
    };
  }

  const requiresHeaderStateReset = blockHeaders.length > 0
    || lastSyncedHeaderHeight !== -1
    || headersMetadata.size > 0
    || hashesByHeight.size > 0;

  return {
    blockHeaders: [],
    firstHeaderHeight: -1,
    startBlockHeight: 1,
    requiresHeaderStateReset,
  };
}

module.exports = deriveBlockHeadersResumeContext;

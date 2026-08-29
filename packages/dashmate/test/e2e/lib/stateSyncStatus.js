import DAPIClient from '@dashevo/dapi-client';
import wait from '../../../src/util/wait.js';
import { getDapiAddress } from './platformSdk.js';

/**
 * Observation helpers for a node that is (or has just finished) state syncing.
 *
 * Tenderdash's own RPC is the source of truth: rs-dapi's `getStatus` copies the
 * state sync counters straight out of `sync_info`, and Drive reports none of
 * them. Both paths are exercised here because the DAPI one is what operators
 * actually reach for.
 */

/**
 * Fetch `sync_info` from a node's Tenderdash RPC.
 *
 * @param {Config} config
 * @return {Promise<Object>}
 */
export async function getTenderdashSyncInfo(config) {
  let host = config.get('platform.drive.tenderdash.rpc.host');

  if (host === '0.0.0.0') {
    host = '127.0.0.1';
  }

  const port = config.get('platform.drive.tenderdash.rpc.port');

  const response = await fetch(`http://${host}:${port}/status`);

  const { result, sync_info: syncInfo } = await response.json();

  // Tenderdash wraps the response into `result` over HTTP JSON RPC
  return result ? result.sync_info : syncInfo;
}

/**
 * State sync counters Tenderdash reports while restoring a snapshot. rs-dapi
 * surfaces exactly these as `GetStatusResponseV0.StateSync`.
 *
 * @type {string[]}
 */
const STATE_SYNC_FIELDS = [
  'total_synced_time',
  'remaining_time',
  'total_snapshots',
  'chunk_process_avg_time',
  'snapshot_height',
  'snapshot_chunks_count',
  'backfilled_blocks',
  'backfill_blocks_total',
];

/**
 * Pick the state sync counters out of a `sync_info`, keeping only the ones
 * that carry a meaningful (non-zero, non-empty) value.
 *
 * @param {Object} syncInfo
 * @return {Object}
 */
export function pickStateSyncFields(syncInfo) {
  const populated = {};

  STATE_SYNC_FIELDS.forEach((field) => {
    const value = syncInfo[field];

    if (value === undefined || value === null || value === '' || value === '0' || value === 0) {
      return;
    }

    populated[field] = value;
  });

  return populated;
}

/**
 * Ask a node's DAPI for its status.
 *
 * Returns the raw error instead of throwing: this runs against a node that is
 * mid-sync and may not be serving DAPI yet, and a failed observation is data
 * rather than a test failure.
 *
 * @param {Config} config
 * @return {Promise<{ ok: boolean, stateSync?: Object, chain?: Object, error?: string }>}
 */
export async function getDapiStatus(config) {
  const client = new DAPIClient({
    dapiAddresses: [getDapiAddress(config)],
    network: 'regtest',
  });

  try {
    const response = await client.platform.getStatus();

    const stateSync = response.getStateSync();
    const chain = response.getChain();

    return {
      ok: true,
      stateSync: {
        snapshotHeight: stateSync.getSnapshotHeight().toString(),
        snapshotChunksCount: stateSync.getSnapshotChunksCount().toString(),
        totalSnapshots: stateSync.getTotalSnapshots(),
        totalSyncedTime: stateSync.getTotalSyncedTime().toString(),
        chunkProcessAverageTime: stateSync.getChunkProcessAverageTime().toString(),
        backfilledBlocks: stateSync.getBackfilledBlocks().toString(),
        backfillBlocksTotal: stateSync.getBackfillBlocksTotal().toString(),
      },
      chain: {
        catchingUp: chain.isCatchingUp(),
        latestBlockHeight: chain.getLatestBlockHeight().toString(),
        earliestBlockHeight: chain.getEarliestBlockHeight().toString(),
      },
    };
  } catch (error) {
    return { ok: false, error: error.message };
  } finally {
    await client.disconnect().catch(() => {});
  }
}

/**
 * Watch a joining node until it finishes syncing, recording every distinct
 * state sync observation seen along the way.
 *
 * Both transports are polled: Tenderdash RPC (which always carries the
 * counters) and DAPI `getStatus` (which is what an operator would use). The
 * loop ends when the node reports `catching_up: false`, or when the deadline
 * passes.
 *
 * @param {Config} config
 * @param {Object} [options]
 * @param {number} [options.timeoutMs]
 * @param {number} [options.intervalMs]
 * @param {function(string): void} [options.log]
 * @return {Promise<{
 *   syncInfo: Object|undefined,
 *   tenderdashObservations: Object[],
 *   dapiObservations: Object[],
 *   dapiErrors: string[],
 * }>}
 */
export async function watchStateSync(config, {
  timeoutMs = 20 * 60 * 1000,
  intervalMs = 2000,
  log = () => {},
} = {}) {
  const deadline = Date.now() + timeoutMs;

  const tenderdashObservations = [];
  const dapiObservations = [];
  const dapiErrors = new Set();

  let syncInfo;
  let lastSeen = '';

  while (Date.now() < deadline) {
    try {
      syncInfo = await getTenderdashSyncInfo(config);

      if (syncInfo) {
        const populated = pickStateSyncFields(syncInfo);

        // Only record a change, so a slow sync does not produce hundreds of
        // identical rows.
        const fingerprint = JSON.stringify(populated);

        if (Object.keys(populated).length > 0 && fingerprint !== lastSeen) {
          lastSeen = fingerprint;

          tenderdashObservations.push({
            at: new Date().toISOString(),
            catchingUp: syncInfo.catching_up,
            latestBlockHeight: syncInfo.latest_block_height,
            earliestBlockHeight: syncInfo.earliest_block_height,
            ...populated,
          });

          log(`state sync observation: ${fingerprint}`);
        }

        if (syncInfo.catching_up === false
          && parseInt(syncInfo.latest_block_height, 10) > 0) {
          break;
        }
      }
    } catch {
      // Tenderdash RPC is not reachable yet
    }

    const dapiStatus = await getDapiStatus(config);

    if (dapiStatus.ok) {
      dapiObservations.push({ at: new Date().toISOString(), ...dapiStatus });
    } else {
      dapiErrors.add(dapiStatus.error);
    }

    await wait(intervalMs);
  }

  return {
    syncInfo,
    tenderdashObservations,
    dapiObservations,
    dapiErrors: [...dapiErrors],
  };
}

/**
 * Lines worth putting in the run report: the whole sync lifecycle, including
 * the peering and consensus handover around it.
 *
 * @type {RegExp}
 */
const LIFECYCLE_LOG_PATTERN = /snapshot|statesync|state sync|state_sync|chunk|backfill|switching to consensus|added peer|handshake/i;

/**
 * Lines only a node that actually state synced can emit. `added peer`,
 * `handshake` and `switching to consensus` appear on every Tenderdash start
 * regardless of how the node caught up, so they must not back an assertion
 * that a state sync happened.
 *
 * @type {RegExp}
 */
const STATE_SYNC_LOG_PATTERN = /snapshot|statesync|state_sync|state sync|chunk|backfill/i;

/**
 * Pull the Tenderdash log lines that trace a state sync from offer to
 * completion, for the run report.
 *
 * @param {DockerCompose} dockerCompose
 * @param {Config} config
 * @param {number} [tail]
 * @return {Promise<{ lines: string[], stateSyncLines: string[] }>}
 */
export async function getStateSyncLogExcerpt(dockerCompose, config, tail = 4000) {
  let output;

  try {
    ({ out: output } = await dockerCompose.logs(config, ['drive_tenderdash'], { tail }));
  } catch (error) {
    return {
      lines: [`unable to read drive_tenderdash logs: ${error.message}`],
      stateSyncLines: [],
    };
  }

  const all = output.split('\n').map((line) => line.trimEnd());

  return {
    lines: all.filter((line) => LIFECYCLE_LOG_PATTERN.test(line)),
    stateSyncLines: all.filter((line) => STATE_SYNC_LOG_PATTERN.test(line)),
  };
}

/**
 * The height drive-abci reports after restoring a snapshot, or undefined when
 * it never restored one.
 *
 * This is the direct evidence that a node state synced. `earliest_block_height`
 * is not: after a successful restore Tenderdash backfills light blocks
 * *backwards* from the snapshot height to satisfy its evidence window, and on
 * a short chain that backfill reaches genesis — so a node that demonstrably
 * restored a snapshot still ends up reporting 1, exactly like a node that
 * replayed every block. The ABCI app saying it completed a restore cannot be
 * produced by block execution, so it distinguishes the two paths cleanly.
 *
 * `since` scopes the read to one boot. Without it a node that restored once
 * and was later wiped and restarted would still show the first restore, and
 * the assertion that it re-synced would pass whether or not it actually did —
 * silently excusing exactly the regression that scenario exists to catch.
 *
 * A failure to read the logs throws rather than reporting "no restore": the
 * fallback scenario asserts on the absence of a restore, and a docker hiccup
 * must not be able to masquerade as proof of it.
 *
 * @param {DockerCompose} dockerCompose
 * @param {Config} config
 * @param {Object} [options]
 * @param {number} [options.tail]
 * @param {string} [options.since] - RFC3339 time to read from
 * @return {Promise<number|undefined>}
 */
export async function getStateSyncRestoreHeight(dockerCompose, config, {
  tail = 4000,
  since,
} = {}) {
  const { out: output } = await dockerCompose.logs(config, ['drive_abci'], { tail, since });

  // drive-abci's tracing output colours its field names, so the literal text
  // is `state_sync completed <esc>height<esc>=<esc>28`. Strip the escapes
  // before matching or the height never parses.
  // eslint-disable-next-line no-control-regex
  const plain = output.replace(/\u001B\[[0-9;]*m/g, '');

  // Matched within one line rather than assuming `height` sits immediately
  // after the message, so adding a field to that tracing call cannot silently
  // turn a successful restore into "never restored".
  const matches = [...plain.matchAll(/state_sync completed[^\n]*?height\s*=\s*(\d+)/g)];

  if (matches.length === 0) {
    return undefined;
  }

  return parseInt(matches[matches.length - 1][1], 10);
}

/**
 * Raw tail of one service's logs, for when a node fails in a way the filtered
 * state sync excerpt cannot explain.
 *
 * @param {DockerCompose} dockerCompose
 * @param {Config} config
 * @param {string} serviceName
 * @param {number} [tail]
 * @return {Promise<string[]>}
 */
export async function getServiceLogTail(dockerCompose, config, serviceName, tail = 80) {
  try {
    const { out } = await dockerCompose.logs(config, [serviceName], { tail });

    return out.split('\n').map((line) => line.trimEnd()).filter(Boolean);
  } catch (error) {
    return [`unable to read ${serviceName} logs: ${error.message}`];
  }
}

/**
 * State of every container of a node, for diagnosing a service that came up
 * and then died.
 *
 * @param {DockerCompose} dockerCompose
 * @param {Config} config
 * @return {Promise<string[]>}
 */
export async function getContainerStates(dockerCompose, config) {
  try {
    const list = await dockerCompose.getContainersList(config, { all: true });

    return list.map((entry) => `${entry.Service || entry.Name}: ${entry.State}`);
  } catch (error) {
    return [`unable to list containers: ${error.message}`];
  }
}

/**
 * Block until a joining node reports that a snapshot restore is genuinely
 * under way, so a caller can disturb the network at a moment that matters.
 *
 * Returns the observation that proved it, or undefined when the node finished
 * (or never started) syncing first — the caller decides whether that makes
 * its scenario inconclusive rather than failed.
 *
 * @param {Config} config
 * @param {Object} [options]
 * @param {number} [options.timeoutMs]
 * @param {number} [options.intervalMs]
 * @return {Promise<Object|undefined>}
 */
export async function waitForStateSyncActivity(config, {
  timeoutMs = 10 * 60 * 1000,
  intervalMs = 1000,
} = {}) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      const syncInfo = await getTenderdashSyncInfo(config);

      if (syncInfo) {
        const populated = pickStateSyncFields(syncInfo);

        // A snapshot height means an offer was accepted; the chunk counters
        // mean data is actually moving.
        if (populated.snapshot_height
          || populated.snapshot_chunks_count
          || populated.chunk_process_avg_time) {
          return populated;
        }

        if (syncInfo.catching_up === false
          && parseInt(syncInfo.latest_block_height, 10) > 0) {
          return undefined;
        }
      }
    } catch {
      // Tenderdash RPC is not reachable yet
    }

    await wait(intervalMs);
  }

  return undefined;
}

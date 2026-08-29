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
 * Pull the Tenderdash log lines that trace a state sync from offer to
 * completion, for the run report.
 *
 * @param {DockerCompose} dockerCompose
 * @param {Config} config
 * @param {number} [tail]
 * @return {Promise<string[]>}
 */
export async function getStateSyncLogExcerpt(dockerCompose, config, tail = 4000) {
  let output;

  try {
    ({ out: output } = await dockerCompose.logs(config, ['drive_tenderdash'], { tail }));
  } catch (error) {
    return [`unable to read drive_tenderdash logs: ${error.message}`];
  }

  const interesting = /snapshot|statesync|state sync|state_sync|chunk|backfill|switching to consensus|added peer|handshake/i;

  return output
    .split('\n')
    .filter((line) => interesting.test(line))
    .map((line) => line.trimEnd());
}

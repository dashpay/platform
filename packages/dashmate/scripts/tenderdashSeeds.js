import { isIPv4 } from 'net';
import seedSetHash from '../src/tenderdash/seedSetHash.js';

const CHAINS = { mainnet: 'main', testnet: 'test' };
export const QUORUM_URLS = {
  mainnet: 'https://quorums.mainnet.networks.dash.org/masternodes',
  testnet: 'https://quorums.testnet.networks.dash.org/masternodes',
};
const MIN_SEEDS = 5;
const MAX_SEEDS = 20;
const MAX_AGE_SECONDS = 7 * 24 * 60 * 60;

/**
 * Validate persisted Core or quorum-server provenance and release freshness.
 * @param {Object} snapshot
 * @param {string} network
 * @param {number} now seconds since epoch
 */
export function checkSnapshot(snapshot, network, now = Date.now() / 1000) {
  const timestamp = snapshot.source === undefined ? snapshot.blockTime : snapshot.lastUpdated;
  const validSource = snapshot.source === undefined
    ? Number.isSafeInteger(snapshot.height) && snapshot.height > 0
      && /^[a-f0-9]{64}$/.test(snapshot.blockHash)
    : snapshot.source === QUORUM_URLS[network];
  if (snapshot.chain !== CHAINS[network] || !validSource
    || !Number.isSafeInteger(timestamp)
    || timestamp > now + 3600 || now - timestamp > MAX_AGE_SECONDS
    || !Array.isArray(snapshot.seeds) || snapshot.seeds.length < MIN_SEEDS
    || snapshot.seeds.length > MAX_SEEDS
    || !Array.isArray(snapshot.previousSeedSetHashes)
    || snapshot.previousSeedSetHashes.some(hash => !/^[a-f0-9]{64}$/.test(hash))) {
    throw new Error(`${network}: invalid or stale seed snapshot; regenerate before releasing`);
  }
  const ids = new Set();
  const hosts = new Set();
  for (const { id, host, port } of snapshot.seeds) {
    if (typeof id !== 'string' || !/^[a-f0-9]{40}$/.test(id) || /^0+$/.test(id) || !isIPv4(host)
      || !Number.isInteger(port) || port < 1 || port > 65535
      || ids.has(id) || hosts.has(host)) {
      throw new Error(`${network}: invalid or duplicate seed`);
    }
    ids.add(id);
    hosts.add(host);
  }
}

/**
 * Select deterministic, unique IPv4 bootstrap peers from registered endpoints.
 * @param {Array} nodes
 * @returns {Array}
 */
function selectSeeds(nodes) {
  const candidates = nodes.flatMap(node => {
    const host = typeof node.address === 'string' && node.address.match(/^([0-9.]+):[0-9]+$/)?.[1];
    const addresses = node.addresses?.platform_p2p ?? [`${host}:${node.platformP2PPort}`];
    if (!Array.isArray(addresses)) { return []; }
    return addresses.flatMap(address => {
      const match = typeof address === 'string' && address.match(/^([0-9.]+):([0-9]+)$/);
      return match ? [{ id: node.platformNodeID, host: match[1], port: Number(match[2]) }] : [];
    });
  })
    .filter(({ id, host, port }) => typeof id === 'string' && /^[a-f0-9]{40}$/.test(id) && !/^0+$/.test(id)
      && isIPv4(host) && Number.isInteger(port) && port > 0 && port <= 65535)
    .sort((a, b) => `${a.id}@${a.host}:${a.port}`.localeCompare(`${b.id}@${b.host}:${b.port}`));
  const seeds = [];
  for (const candidate of candidates) {
    if (!seeds.some(seed => seed.id === candidate.id || seed.host === candidate.host)) {
      seeds.push(candidate);
    }
    if (seeds.length === MAX_SEEDS) { break; }
  }
  return seeds;
}

/**
 * Preserve migration history regardless of which registry source is used.
 * @param {Object} snapshot
 * @param {Object} previous
 * @param {string} network
 * @param {number} now
 * @returns {Object}
 */
function finishSnapshot(snapshot, previous, network, now) {
  const result = {
    ...snapshot,
    previousSeedSetHashes: [...new Set([
      ...previous.previousSeedSetHashes,
      seedSetHash(previous.seeds),
    ])].sort(),
  };
  checkSnapshot(result, network, now);
  return result;
}

/**
 * Generate at one Core block; use registered Platform endpoints, never guessed ports.
 * @param {function(string, ...string): Object} rpc
 * @param {string} network
 * @param {Object} previous
 * @param {number} now seconds since epoch
 * @returns {Object}
 */
export function generateSnapshot(rpc, network, previous, now = Date.now() / 1000) {
  const info = rpc('getblockchaininfo');
  if (info.chain !== CHAINS[network] || info.initialblockdownload !== false
    || !Number.isSafeInteger(info.blocks) || info.blocks !== info.headers
    || !Number.isSafeInteger(info.time) || now - info.time > 24 * 60 * 60) {
    throw new Error(`${network}: Core must be synced to the correct network's recent tip`);
  }
  const registry = rpc('protx', 'list', 'valid', 'true', String(info.blocks));
  if (!Array.isArray(registry)) {
    throw new Error(`${network}: expected a detailed protx registry`);
  }
  if (rpc('getblockhash', String(info.blocks)) !== info.bestblockhash) {
    throw new Error(`${network}: Core reorganized during generation; retry`);
  }
  const seeds = selectSeeds(registry
    .filter(node => node.type === 'Evo' && node.state?.PoSeBanHeight === -1
      && !node.metaInfo?.is_platform_banned)
    .map(({ state }) => ({ ...state, address: state.service })));
  const snapshot = {
    chain: info.chain,
    height: info.blocks,
    blockHash: info.bestblockhash,
    blockTime: info.time,
    seeds,
  };
  return finishSnapshot(snapshot, previous, network, now);
}

/**
 * Consume quorum-list-server's /masternodes contract without fabricating Core block metadata.
 * @param {Object} response
 * @param {string} network
 * @param {Object} previous
 * @param {number} now seconds since epoch
 * @returns {Object}
 */
export function generateQuorumSnapshot(response, network, previous, now = Date.now() / 1000) {
  if (response?.success !== true || !Array.isArray(response.data)
    || !Number.isSafeInteger(response.lastUpdated)
    || response.lastUpdated > now + 300 || now - response.lastUpdated > 30 * 60) {
    throw new Error(`${network}: quorum registry is unavailable, stale, or missing lastUpdated; the server must support bootstrap metadata`);
  }
  const seeds = selectSeeds(response.data.filter(node => node?.status === 'ENABLED'
    && node.versionCheck === 'success'));
  if (seeds.length < MIN_SEEDS) {
    throw new Error(`${network}: quorum registry needs at least ${MIN_SEEDS} eligible nodes with platformNodeID and registered P2P endpoints`);
  }
  return finishSnapshot({
    chain: CHAINS[network],
    source: QUORUM_URLS[network],
    lastUpdated: response.lastUpdated,
    seeds,
  }, previous, network, now);
}

/**
 * Fetch the public registry over HTTPS; failures never fall back to old snapshots.
 * @param {string} network
 * @param {Object} previous
 * @param {function} fetchImpl
 * @param {number} now seconds since epoch
 * @returns {Promise<Object>}
 */
export async function fetchQuorumSnapshot(
  network,
  previous,
  fetchImpl = globalThis.fetch,
  now = Date.now() / 1000,
) {
  const url = QUORUM_URLS[network];
  let response;
  try {
    const result = await fetchImpl(url, { signal: AbortSignal.timeout(15000), redirect: 'error' });
    if (!result.ok) { throw new Error(`HTTP ${result.status}`); }
    response = await result.json();
  } catch (error) {
    throw new Error(`${network}: failed to fetch ${url}: ${error.message}`);
  }
  return generateQuorumSnapshot(response, network, previous, now);
}

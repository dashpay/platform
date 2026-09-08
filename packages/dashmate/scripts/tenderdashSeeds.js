import { isIPv4 } from 'net';
import seedSetHash from '../src/tenderdash/seedSetHash.js';

const CHAINS = { mainnet: 'main', testnet: 'test' };
const MIN_SEEDS = 5;
const MAX_SEEDS = 20;
const MAX_AGE_SECONDS = 7 * 24 * 60 * 60;

/**
 * Reject snapshots from unsynced nodes, another chain, or an old release.
 * @param {Object} snapshot
 * @param {string} network
 * @param {number} now seconds since epoch
 */
export function checkSnapshot(snapshot, network, now = Date.now() / 1000) {
  if (snapshot.chain !== CHAINS[network]
    || !Number.isSafeInteger(snapshot.height) || snapshot.height <= 0
    || !/^[a-f0-9]{64}$/.test(snapshot.blockHash)
    || !Number.isSafeInteger(snapshot.blockTime)
    || snapshot.blockTime > now + 3600 || now - snapshot.blockTime > MAX_AGE_SECONDS
    || !Array.isArray(snapshot.seeds) || snapshot.seeds.length < MIN_SEEDS
    || snapshot.seeds.length > MAX_SEEDS
    || !Array.isArray(snapshot.previousSeedSetHashes)
    || snapshot.previousSeedSetHashes.some(hash => !/^[a-f0-9]{64}$/.test(hash))) {
    throw new Error(`${network}: invalid or stale seed snapshot; regenerate from synced Core nodes`);
  }
  const ids = new Set();
  const hosts = new Set();
  for (const { id, host, port } of snapshot.seeds) {
    if (!/^[a-f0-9]{40}$/.test(id) || /^0+$/.test(id) || !isIPv4(host)
      || !Number.isInteger(port) || port < 1 || port > 65535
      || ids.has(id) || hosts.has(host)) {
      throw new Error(`${network}: invalid or duplicate seed`);
    }
    ids.add(id);
    hosts.add(host);
  }
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
  const candidates = registry
    .filter(node => node.type === 'Evo' && node.state?.PoSeBanHeight === -1
      && !node.metaInfo?.is_platform_banned)
    .flatMap(({ state }) => {
      // Core v23 advertises independent Platform endpoints; older versions use service's IP.
      const addresses = state.addresses?.platform_p2p
        ?? [`${state.service?.split(':')[0]}:${state.platformP2PPort}`];
      return addresses.map(address => {
        const [host, port] = address.split(':');
        return { id: state.platformNodeID, host, port: Number(port) };
      });
    })
    .filter(({ id, host, port }) => /^[a-f0-9]{40}$/.test(id) && !/^0+$/.test(id)
      && isIPv4(host) && Number.isInteger(port) && port > 0 && port <= 65535)
    .sort((a, b) => `${a.id}@${a.host}:${a.port}`.localeCompare(`${b.id}@${b.host}:${b.port}`));
  const seeds = [];
  for (const candidate of candidates) {
    if (!seeds.some(seed => seed.id === candidate.id || seed.host === candidate.host)) {
      seeds.push(candidate);
    }
    if (seeds.length === MAX_SEEDS) { break; }
  }
  const snapshot = {
    chain: info.chain,
    height: info.blocks,
    blockHash: info.bestblockhash,
    blockTime: info.time,
    seeds,
    previousSeedSetHashes: [...new Set([
      ...previous.previousSeedSetHashes,
      seedSetHash(previous.seeds),
    ])].sort(),
  };
  checkSnapshot(snapshot, network, now);
  return snapshot;
}

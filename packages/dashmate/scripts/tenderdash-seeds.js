import { isIPv4 } from 'net';
import crypto from 'crypto';
import seedSetHash from '../src/tenderdash/seedSetHash.js';

export const QUORUM_URLS = {
  mainnet: 'https://quorums.mainnet.networks.dash.org/masternodes',
  testnet: 'https://quorums.testnet.networks.dash.org/masternodes',
};
const MIN_SEEDS = 5;
const MAX_SEEDS = 20;
export const MAX_AGE_SECONDS = 7 * 24 * 60 * 60;

/** Validate an IPv4 Tenderdash identity and registered port. */
function validSeed({ id, host, port }) {
  return typeof id === 'string' && /^[a-f0-9]{40}$/.test(id) && !/^0+$/.test(id)
    && isIPv4(host) && Number.isInteger(port) && port > 0 && port <= 65535;
}

/**
 * Validate quorum-server provenance and release freshness.
 * @param {Object} snapshot
 * @param {string} network
 * @param {number} now seconds since epoch
 */
export function checkSnapshot(snapshot, network, now = Date.now() / 1000) {
  if (snapshot.source !== QUORUM_URLS[network]
    || !Number.isSafeInteger(snapshot.lastUpdated)
    || snapshot.lastUpdated > now + 300 || now - snapshot.lastUpdated > MAX_AGE_SECONDS
    || !Array.isArray(snapshot.seeds) || snapshot.seeds.length < MIN_SEEDS
    || snapshot.seeds.length > MAX_SEEDS
    || !Array.isArray(snapshot.previousSeedSetHashes)
    || snapshot.previousSeedSetHashes.some(hash => !/^[a-f0-9]{64}$/.test(hash))) {
    throw new Error(`${network}: invalid or stale seed snapshot; regenerate before releasing`);
  }
  const ids = new Set();
  const hosts = new Set();
  for (const { id, host, port } of snapshot.seeds) {
    if (!validSeed({ id, host, port }) || ids.has(id) || hosts.has(host)) {
      throw new Error(`${network}: invalid or duplicate seed`);
    }
    ids.add(id);
    hosts.add(host);
  }
}

/**
 * Sample unique IPv4 bootstrap peers after fixing the eligible registry.
 * @param {Array} nodes
 * @returns {Array}
 */
function selectSeeds(nodes) {
  const candidatesById = new Map();
  for (const node of nodes) {
    const host = typeof node.address === 'string' && node.address.match(/^([0-9.]+):[0-9]+$/)?.[1];
    const addresses = node.addresses?.platform_p2p ?? [`${host}:${node.platformP2PPort}`];
    if (!Array.isArray(addresses)) { continue; }
    const endpoints = addresses.map(address => {
      const match = typeof address === 'string' && address.match(/^([0-9.]+):([0-9]+)$/);
      return match && { id: node.platformNodeID, host: match[1], port: Number(match[2]) };
    }).filter(seed => seed && validSeed(seed));
    if (endpoints.length > 0 && !candidatesById.has(node.platformNodeID)) {
      candidatesById.set(node.platformNodeID, endpoints);
    }
  }
  // One entry per identity: extra endpoints must not buy extra sampling chances.
  const candidates = [...candidatesById.values()];
  for (let i = candidates.length - 1; i > 0; i--) {
    const j = crypto.randomInt(i + 1);
    [candidates[i], candidates[j]] = [candidates[j], candidates[i]];
  }
  // Match identities to hosts in sampled order. An identity whose hosts are all
  // taken may move an earlier pick to one of its alternate endpoints, so a
  // registry that admits a host-disjoint assignment always gets one. Earlier
  // picks keep their slot; only the endpoint they are listed under can change.
  const seedsByHost = new Map();
  const assign = (endpoints, visited) => endpoints.some(endpoint => {
    if (visited.has(endpoint.host)) { return false; }
    visited.add(endpoint.host);
    const holder = seedsByHost.get(endpoint.host);
    if (holder && !assign(holder.endpoints, visited)) { return false; }
    seedsByHost.set(endpoint.host, { endpoints, endpoint });
    return true;
  });
  for (const endpoints of candidates) {
    if (seedsByHost.size === MAX_SEEDS) { break; }
    assign(endpoints, new Set());
  }
  return [...seedsByHost.values()].map(({ endpoint }) => endpoint)
    .sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * Generate a release snapshot from quorum-list-server's /masternodes response.
 * @param {Object} response
 * @param {string} network
 * @param {Object} previous
 * @param {number} now seconds since epoch
 * @returns {Object}
 */
export function generateSnapshot(response, network, previous, now = Date.now() / 1000) {
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
  const snapshot = {
    source: QUORUM_URLS[network],
    lastUpdated: response.lastUpdated,
    seeds,
    previousSeedSetHashes: [...new Set([
      ...previous.previousSeedSetHashes, seedSetHash(previous.seeds),
    ])].sort(),
  };
  checkSnapshot(snapshot, network, now);
  return snapshot;
}

/**
 * Fetch the public registry over HTTPS; failures never fall back to old snapshots.
 * @param {string} network
 * @param {Object} previous
 * @param {function} fetchImpl
 * @param {number} now seconds since epoch
 * @returns {Promise<Object>}
 */
export async function fetchSnapshot(
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
  return generateSnapshot(response, network, previous, now);
}

import { createHash } from 'crypto';

/**
 * Identify a shipped default set without treating ordering or duplicates as customization.
 * @param {Array} seeds
 * @param {string|null} omittedNodeId bind a filtered stock list to the node it is for
 * @returns {string}
 */
export default function seedSetHash(seeds, omittedNodeId = null) {
  const entries = [...new Set(seeds.map(({ id, host, port }) => `${id}@${host}:${port}`))].sort();
  const fingerprint = omittedNodeId === null ? entries : [omittedNodeId, entries];
  return createHash('sha256').update(JSON.stringify(fingerprint)).digest('hex');
}

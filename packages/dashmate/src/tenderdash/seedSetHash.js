import { createHash } from 'crypto';

/**
 * Identify a shipped default set without treating ordering or duplicates as customization.
 * @param {Array} seeds
 * @returns {string}
 */
export default function seedSetHash(seeds) {
  const entries = [...new Set(seeds.map(({ id, host, port }) => `${id}@${host}:${port}`))].sort();
  return createHash('sha256').update(JSON.stringify(entries)).digest('hex');
}

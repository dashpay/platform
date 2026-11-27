import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';
import validateTenderdashNodeKey from './validateTenderdashNodeKey.js';

/** Return attributable node IDs, with the running identity first. */
export function getTenderdashSeedNodeIds(node) {
  const { id, key } = node ?? {};
  const derivedId = typeof key === 'string' && validateTenderdashNodeKey(key) === true
    ? deriveTenderdashNodeId(key)
    : null;
  const savedId = typeof id === 'string' ? id.toLowerCase() : null;

  return [...new Set([derivedId, savedId].filter(Boolean))];
}

/** Omit self while preserving the order and spelling of the other peers. */
export default function filterTenderdashSeeds(seeds, nodeId) {
  return (Array.isArray(seeds) ? seeds : []).filter(({ id }) => (
    nodeId === null || typeof id !== 'string' || id.toLowerCase() !== nodeId
  ));
}

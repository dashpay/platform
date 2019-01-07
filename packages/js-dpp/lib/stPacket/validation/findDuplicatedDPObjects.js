/**
 * @param {Object} rawDPObject
 * @return {string}
 */
function createFingerPrint(rawDPObject) {
  return [
    rawDPObject.$type,
    rawDPObject.$scope,
    rawDPObject.$scopeId,
  ].join(':');
}

/**
 * Find duplicates
 *
 * @typedef findDuplicatedDPObjects
 * @param {Object[]} rawDPObjects
 * @return {Object[]}
 */
function findDuplicatedDPObjects(rawDPObjects) {
  const fingerprints = {};
  const duplicates = new Set();

  rawDPObjects.forEach((rawDPObject) => {
    const fingerprint = createFingerPrint(rawDPObject);

    if (!fingerprints[fingerprint]) {
      fingerprints[fingerprint] = [];
    }

    fingerprints[fingerprint].push(rawDPObject);

    if (fingerprints[fingerprint].length > 1) {
      fingerprints[fingerprint].forEach(o => duplicates.add(o));
    }
  });

  return Array.from(duplicates);
}

module.exports = findDuplicatedDPObjects;

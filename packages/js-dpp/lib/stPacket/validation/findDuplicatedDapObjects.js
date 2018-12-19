/**
 * @param {Object} rawDapObject
 * @return {string}
 */
function createFingerPrint(rawDapObject) {
  return [
    rawDapObject.$type,
    rawDapObject.$scope,
    rawDapObject.$scopeId,
  ].join(':');
}

/**
 * Find duplicates
 *
 * @param {Object[]} rawDapObjects
 * @return {Object[]}
 */
function findDuplicatedDapObjects(rawDapObjects) {
  const fingerprints = {};
  const duplicates = new Set();

  rawDapObjects.forEach((rawDapObject) => {
    const fingerprint = createFingerPrint(rawDapObject);

    if (!fingerprints[fingerprint]) {
      fingerprints[fingerprint] = [];
    }

    fingerprints[fingerprint].push(rawDapObject);

    if (fingerprints[fingerprint].length > 1) {
      fingerprints[fingerprint].forEach(o => duplicates.add(o));
    }
  });

  return Array.from(duplicates);
}

module.exports = findDuplicatedDapObjects;

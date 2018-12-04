const findDuplicates = require('../../util/findDuplicates');

/**
 * @param {Object} dapObject
 * @return {string}
 */
function createFingerPrint(dapObject) {
  return [
    dapObject.$type,
    dapObject.$scope,
    dapObject.$scopeId,
  ].join(':');
}

/**
 * Find duplicates
 *
 * @param {Object[]} dapObjects
 * @return {Object[]}
 */
function findDuplicatedDapObjects(dapObjects) {
  const dapObjectFingerprints = {};

  dapObjects.forEach((dapObject) => {
    dapObjectFingerprints[createFingerPrint(dapObject)] = dapObject;
  });

  const duplicates = findDuplicates(
    Object.keys(dapObjectFingerprints),
  );

  return duplicates.map(fingerprint => dapObjectFingerprints[fingerprint]);
}

module.exports = findDuplicatedDapObjects;

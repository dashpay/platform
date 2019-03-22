const Document = require('../../document/Document');

/**
 * @param {Document} document
 * @return {string}
 */
function createFingerPrint(document) {
  return [
    document.getType(),
    document.getId(),
  ].join(':');
}

/**
 * Find duplicates
 *
 * @typedef findDuplicateDocuments
 * @param {RawDocument[]} rawDocuments
 * @return {RawDocument[]}
 */
function findDuplicateDocuments(rawDocuments) {
  const fingerprints = {};
  const duplicates = [];

  rawDocuments
    .map(o => new Document(o))
    .forEach((document) => {
      const fingerprint = createFingerPrint(document);

      if (!fingerprints[fingerprint]) {
        fingerprints[fingerprint] = [];
      }

      fingerprints[fingerprint].push(document.toJSON());

      if (fingerprints[fingerprint].length > 1) {
        duplicates.push(...fingerprints[fingerprint]);
      }
    });

  return duplicates;
}

module.exports = findDuplicateDocuments;

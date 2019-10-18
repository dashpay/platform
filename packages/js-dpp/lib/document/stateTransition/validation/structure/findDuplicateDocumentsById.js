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
 * @typedef findDuplicateDocumentsById
 * @param {Document[]} documents
 * @return {RawDocument[]}
 */
function findDuplicateDocumentsById(documents) {
  const fingerprints = {};
  const duplicates = [];

  documents
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

module.exports = findDuplicateDocumentsById;

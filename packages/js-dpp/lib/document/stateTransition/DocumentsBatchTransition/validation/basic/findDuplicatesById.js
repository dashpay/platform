/**
 * @param {
 *   RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition
 * } documentTransition
 *
 * @return {string}
 */
function createFingerPrint(documentTransition) {
  return [
    documentTransition.$type,
    documentTransition.$id,
  ].join(':');
}

/**
 * Find duplicates
 *
 * @typedef findDuplicatesById
 *
 * @param {
 *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
 * } documentTransitions
 *
 * @return {
 *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
 * }
 */
function findDuplicatesById(documentTransitions) {
  const fingerprints = {};
  const duplicates = [];

  documentTransitions
    .forEach((documentTransition) => {
      const fingerprint = createFingerPrint(documentTransition);

      if (!fingerprints[fingerprint]) {
        fingerprints[fingerprint] = [];
      }

      fingerprints[fingerprint].push(documentTransition);

      if (fingerprints[fingerprint].length > 1) {
        duplicates.push(...fingerprints[fingerprint]);
      }
    });

  return duplicates;
}

module.exports = findDuplicatesById;

const documentsBatchSchema = require('@dashevo/dpp/schema/document/stateTransition/documentsBatch.json');

/**
 * @typedef splitDocumentsIntoChunks
 * @param {Object} documents
 * @param {Document[]} [documents.create]
 * @param {Document[]} [documents.delete]
 * @return {Object[]}
 */
function splitDocumentsIntoChunks(documents) {
  const chunkedDocuments = [];
  const { create: documentsToCreate, delete: documentsToDelete } = documents;

  const maxLength = Math.max(documentsToCreate.length, documentsToDelete.length);
  const chunk = documentsBatchSchema.properties.transitions.maxItems;
  if (maxLength <= chunk) {
    return [documents];
  }

  for (let i = 0; i < maxLength; i += chunk) {
    const result = {};

    if (documentsToCreate.length > i) {
      result.create = documentsToCreate.slice(i, i + chunk);
    }

    if (documentsToDelete.length > i) {
      result.delete = documentsToDelete.slice(i, i + chunk);
    }

    chunkedDocuments.push(result);
  }

  return chunkedDocuments;
}

module.exports = splitDocumentsIntoChunks;

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

  const chunkLength = documentsBatchSchema.properties.transitions.maxItems;

  if (documentsToCreate.length + documentsToDelete.length <= chunkLength) {
    return [documents];
  }

  for (let i = 0; i < documentsToCreate.length; i += chunkLength) {
    const chunk = {
      create: documentsToCreate.slice(i, i + chunkLength),
    };

    chunkedDocuments.push(chunk);
  }

  const lastCreateChunkLength = chunkedDocuments[chunkedDocuments.length - 1].create.length;
  const documentsToAddLength = chunkLength - lastCreateChunkLength;

  chunkedDocuments[chunkedDocuments.length - 1].delete = documentsToDelete.slice(
    0,
    documentsToAddLength,
  );

  for (let i = documentsToAddLength; i < documentsToDelete.length; i += chunkLength) {
    const chunk = {
      delete: documentsToDelete.slice(i, i + chunkLength),
    };

    chunkedDocuments.push(chunk);
  }

  return chunkedDocuments;
}

module.exports = splitDocumentsIntoChunks;

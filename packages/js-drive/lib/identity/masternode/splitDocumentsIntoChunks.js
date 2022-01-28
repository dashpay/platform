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

  const documentsToCreate = documents.create || [];
  const documentsToDelete = documents.delete || [];

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

  let documentsToAddLength = 0;

  if (chunkedDocuments.length > 0) {
    const lastCreateChunkLength = chunkedDocuments[chunkedDocuments.length - 1].create.length;
    documentsToAddLength = chunkLength - lastCreateChunkLength;

    if (documentsToAddLength > 0 && documentsToDelete.length > 0) {
      chunkedDocuments[chunkedDocuments.length - 1].delete = documentsToDelete.slice(
        0,
        documentsToAddLength,
      );
    }
  }

  for (let i = documentsToAddLength; i < documentsToDelete.length; i += chunkLength) {
    const chunk = {
      delete: documentsToDelete.slice(i, i + chunkLength),
    };

    chunkedDocuments.push(chunk);
  }

  return chunkedDocuments;
}

module.exports = splitDocumentsIntoChunks;

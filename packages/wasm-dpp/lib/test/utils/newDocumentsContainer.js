const { default: loadWasmDpp } = require('../../../dist');

/**
 * Creates container with documents
 *
 * @return {DocumentsContainer}
 */
async function newDocumentsContainer(documents) {
  const {
    create: createDocuments,
    replace: replaceDocuments,
    delete: deleteDocuments,
  } = documents;
  const { DocumentsContainer } = await loadWasmDpp();

  const documentsContainer = new DocumentsContainer();
  if (createDocuments != null) {
    createDocuments.forEach((document) => {
      documentsContainer.pushDocumentCreate(document);
    });
  }
  if (replaceDocuments != null) {
    replaceDocuments.forEach((document) => {
      documentsContainer.pushDocumentReplace(document);
    });
  }
  if (deleteDocuments != null) {
    deleteDocuments.forEach((document) => {
      documentsContainer.pushDocumentDelete(document);
    });
  }

  return documentsContainer;
}

module.exports = newDocumentsContainer;

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
		for (const d of createDocuments) {
			documentsContainer.pushDocumentCreate(d);
		}
	}
	if (replaceDocuments != null) {
		for (const d of replaceDocuments) {
			documentsContainer.pushDocumentReplace(d);
		}
	}
	if (deleteDocuments != null) {
		for (const d of deleteDocuments) {
			documentsContainer.pushDeleteDocument(d);
		}
	}

	return documentsContainer;
}

module.exports = newDocumentsContainer;

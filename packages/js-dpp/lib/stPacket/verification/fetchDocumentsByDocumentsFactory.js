/**
 * @param {DataProvider} dataProvider
 * @return {fetchDocumentsByDocuments}
 */
function fetchDocumentsByDocumentsFactory(dataProvider) {
  /**
   * @typedef fetchDocumentsByDocuments
   * @param {string} contractId
   * @param {Document[]} documents
   * @return {Document[]}
   */
  async function fetchDocumentsByDocuments(contractId, documents) {
    // Group Document IDs by types
    const documentIdsByTypes = documents.reduce((obj, document) => {
      if (!obj[document.getType()]) {
        // eslint-disable-next-line no-param-reassign
        obj[document.getType()] = [];
      }

      obj[document.getType()].push(document.getId());

      return obj;
    }, {});

    // Convert object to array
    const documentArray = Object.entries(documentIdsByTypes);

    // Fetch Documents by IDs
    const fetchedDocumentsPromises = documentArray.map(([type, ids]) => {
      const options = {
        where: [['$id', 'in', ids]],
      };

      return dataProvider.fetchDocuments(
        contractId,
        type,
        options,
      );
    });

    const fetchedDocumentsByTypes = await Promise.all(fetchedDocumentsPromises);

    return fetchedDocumentsByTypes.reduce((array, docs) => array.concat(docs), []);
  }

  return fetchDocumentsByDocuments;
}

module.exports = fetchDocumentsByDocumentsFactory;

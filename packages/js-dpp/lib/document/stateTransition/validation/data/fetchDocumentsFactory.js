/**
 * @param {DataProvider} dataProvider
 * @return {fetchDocuments}
 */
function fetchDocumentsFactory(dataProvider) {
  /**
   * @typedef fetchDocuments
   * @param {Document[]} documents
   * @return {Document[]}
   */
  async function fetchDocuments(documents) {
    const dataContractId = documents[0].getDataContractId();

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
        dataContractId,
        type,
        options,
      );
    });

    const fetchedDocumentsByTypes = await Promise.all(fetchedDocumentsPromises);

    return fetchedDocumentsByTypes.reduce((array, docs) => array.concat(docs), []);
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

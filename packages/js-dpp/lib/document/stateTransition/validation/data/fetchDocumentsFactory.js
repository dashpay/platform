/**
 * @param {StateRepository} stateRepository
 * @return {fetchDocuments}
 */
function fetchDocumentsFactory(stateRepository) {
  /**
   * @typedef fetchDocuments
   * @param {string} dataContractId
   * @param {DocumentCreateTransition[]
   *        |DocumentReplaceTransition[]
   *        |DocumentDeleteTransition[]} documentTransitions
   * @return {Document[]}
   */
  async function fetchDocuments(dataContractId, documentTransitions) {
    // Group Document IDs by types
    const documentIdsByTypes = documentTransitions.reduce((obj, document) => {
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

      return stateRepository.fetchDocuments(
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

/**
 * @param {StateRepository} stateRepository
 * @return {fetchDocuments}
 */
function fetchDocumentsFactory(stateRepository) {
  /**
   * @typedef fetchDocuments
   * @param {DocumentCreateTransition[]
   *        |DocumentReplaceTransition[]
   *        |DocumentDeleteTransition[]} documentTransitions
   * @param {StateTransitionExecutionContext} executionContext
   * @return {Promise<Document[]>}
   */
  async function fetchDocuments(documentTransitions, executionContext) {
    // Group document transitions by contracts and types
    const transitionsByContractsAndTypes = documentTransitions.reduce((obj, dt) => {
      const uniqueKey = `${dt.getDataContractId()}${dt.getType()}`;

      if (!obj[uniqueKey]) {
        // eslint-disable-next-line no-param-reassign
        obj[uniqueKey] = [];
      }

      obj[uniqueKey].push(dt);

      return obj;
    }, {});

    // Fetch Documents
    const fetchedDocumentsPromises = Object.values(transitionsByContractsAndTypes)
      .map((transitions) => {
        const options = {
          where: [['$id', 'in', transitions.map((t) => t.getId())]],
          orderBy: [['$id', 'asc']],
        };

        return stateRepository.fetchDocuments(
          transitions[0].getDataContractId(),
          transitions[0].getType(),
          options,
          executionContext,
        );
      });

    const fetchedDocuments = await Promise.all(fetchedDocumentsPromises);

    return fetchedDocuments.reduce((array, docs) => array.concat(docs), []);
  }

  return fetchDocuments;
}

module.exports = fetchDocumentsFactory;

const ValidationResult = require('../../../../validation/ValidationResult');
const DuplicateDocumentError = require('../../../../errors/DuplicateDocumentError');

/**
 * @param {StateRepository} stateRepository
 * @return {validateDocumentsUniquenessByIndices}
 */
function validateDocumentsUniquenessByIndicesFactory(stateRepository) {
  /**
   * @typedef validateDocumentsUniquenessByIndices
   * @param {string} ownerId
   * @param {DocumentCreateTransition[]
   *         |DocumentReplaceTransition[]} documentTransitions
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  async function validateDocumentsUniquenessByIndices(
    ownerId,
    documentTransitions,
    dataContract,
  ) {
    const result = new ValidationResult();

    // 1. Prepare fetchDocuments queries from indexed properties
    const documentIndexQueries = documentTransitions
      .reduce((queries, documentTransition) => {
        const documentSchema = dataContract.getDocumentSchema(documentTransition.getType());

        if (!documentSchema.indices) {
          return queries;
        }

        documentSchema.indices
          .filter((index) => index.unique)
          .forEach((indexDefinition) => {
            const where = indexDefinition.properties
              .map((property) => {
                const propertyName = Object.keys(property)[0];

                let propertyValue;
                if (propertyName === '$ownerId') {
                  propertyValue = ownerId;
                } else {
                  propertyValue = documentTransition.getData()[propertyName];
                }

                return [propertyName, '==', propertyValue];
              });

            queries.push({
              type: documentTransition.getType(),
              indexDefinition,
              documentTransition,
              where,
            });
          });

        return queries;
      }, []);

    // 2. Fetch Document by indexed properties
    const fetchRawDocumentPromises = documentIndexQueries
      .map(({
        type,
        where,
        indexDefinition,
        documentTransition,
      }) => (
        stateRepository.fetchDocuments(
          dataContract.getId(),
          type,
          { where },
        )
          .then((doc) => Object.assign(doc, {
            indexDefinition,
            documentTransition,
          }))
      ));

    const fetchedDocumentsByIndices = await Promise.all(fetchRawDocumentPromises);

    // 3. Create errors if duplicates found
    fetchedDocumentsByIndices
      .filter((docs) => {
        const isEmpty = docs.length === 0;
        const onlyOriginDocument = docs.length === 1
          && docs[0].getId() === docs.documentTransition.getId();

        return !isEmpty && !onlyOriginDocument;
      }).forEach((rawDocuments) => {
        result.addError(
          new DuplicateDocumentError(
            rawDocuments.documentTransition,
            rawDocuments.indexDefinition,
          ),
        );
      });

    return result;
  }

  return validateDocumentsUniquenessByIndices;
}

module.exports = validateDocumentsUniquenessByIndicesFactory;

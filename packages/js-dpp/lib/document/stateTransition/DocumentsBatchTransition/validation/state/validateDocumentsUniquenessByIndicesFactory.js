const ValidationResult = require('../../../../../validation/ValidationResult');
const DuplicateUniqueIndexError = require('../../../../../errors/consensus/state/document/DuplicateUniqueIndexError');
const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

/**
 * @param {StateRepository} stateRepository
 * @return {validateDocumentsUniquenessByIndices}
 */
function validateDocumentsUniquenessByIndicesFactory(stateRepository) {
  /**
   * @typedef validateDocumentsUniquenessByIndices
   * @param {Identifier} ownerId
   * @param {DocumentCreateTransition[]
   *         |DocumentReplaceTransition[]} documentTransitions
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} executionContext
   * @return {ValidationResult}
   */
  async function validateDocumentsUniquenessByIndices(
    ownerId,
    documentTransitions,
    dataContract,
    executionContext,
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
            const where = [];

            indexDefinition.properties.forEach((property) => {
              const propertyName = Object.keys(property)[0];
              let propertyValue;

              switch (propertyName) {
                case '$ownerId':
                  propertyValue = ownerId;
                  break;
                case '$createdAt':
                  if (documentTransition.getAction()
                    === AbstractDocumentTransition.ACTIONS.CREATE) {
                    const createdAt = documentTransition.getCreatedAt();
                    if (createdAt) {
                      propertyValue = createdAt.getTime();
                    }
                  }
                  break;
                case '$updatedAt': {
                  const updatedAt = documentTransition.getUpdatedAt();
                  if (updatedAt) {
                    propertyValue = updatedAt.getTime();
                  }
                }
                  break;
                default:
                  propertyValue = documentTransition.get(propertyName);
              }

              if (propertyValue !== undefined) {
                where.push([propertyName, '==', propertyValue]);
              }
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
      .filter(({ where }) => where.length > 0)
      .map(async ({
        type,
        where,
        indexDefinition,
        documentTransition,
      }) => {
        const doc = await stateRepository.fetchDocuments(
          dataContract.getId(),
          type,
          { where },
          executionContext,
        );

        return Object.assign(doc, {
          indexDefinition,
          documentTransition,
        });
      });

    const fetchedDocumentsByIndices = await Promise.all(fetchRawDocumentPromises);

    if (executionContext.isDryRun()) {
      return result;
    }

    // 3. Create errors if duplicates found
    fetchedDocumentsByIndices
      .filter((docs) => {
        const isEmpty = docs.length === 0;
        const onlyOriginDocument = docs.length === 1
          && docs[0].getId().equals(docs.documentTransition.getId());

        return !isEmpty && !onlyOriginDocument;
      }).forEach((rawDocuments) => {
        result.addError(
          new DuplicateUniqueIndexError(
            rawDocuments.documentTransition.getId().toBuffer(),
            rawDocuments.indexDefinition.properties.map((i) => Object.keys(i)[0]),
          ),
        );
      });

    return result;
  }

  return validateDocumentsUniquenessByIndices;
}

module.exports = validateDocumentsUniquenessByIndicesFactory;

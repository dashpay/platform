const Document = require('../../../Document');

const DataTriggerExecutionContext = require('../../../../dataTrigger/DataTriggerExecutionContext');

const ValidationResult = require('../../../../validation/ValidationResult');

const DocumentAlreadyPresentError = require('../../../../errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../errors/DocumentNotFoundError');
const InvalidDocumentRevisionError = require('../../../../errors/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../errors/InvalidDocumentActionError');

/**
 *
 * @param {DataProvider} dataProvider
 * @param {fetchDocuments} fetchDocuments
 * @param {validateDocumentsUniquenessByIndices} validateDocumentsUniquenessByIndices
 * @param {executeDataTriggers} executeDataTriggers
 * @param {fetchAndValidateDataContract} fetchAndValidateDataContract
 * @return {validateDataContractSTData}
 */
function validateDocumentsSTDataFactory(
  dataProvider,
  fetchDocuments,
  validateDocumentsUniquenessByIndices,
  executeDataTriggers,
  fetchAndValidateDataContract,
) {
  /**
   * @typedef validateDocumentsSTData
   * @param {DocumentsStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsSTData(stateTransition) {
    const result = new ValidationResult();

    const documents = stateTransition.getDocuments();
    const [firstDocument] = documents;

    const userId = firstDocument.getUserId();

    // Data contract must exist
    const dataContractValidationResult = await fetchAndValidateDataContract(firstDocument);
    if (!dataContractValidationResult.isValid()) {
      result.merge(
        dataContractValidationResult,
      );

      return result;
    }

    const dataContract = dataContractValidationResult.getData();

    // Validate document action, userId and revision
    const fetchedDocuments = await fetchDocuments(documents);

    documents
      .forEach((document) => {
        const fetchedDocument = fetchedDocuments.find((d) => document.getId() === d.getId());

        switch (document.getAction()) {
          case Document.ACTIONS.CREATE:
            if (fetchedDocument) {
              result.addError(
                new DocumentAlreadyPresentError(document, fetchedDocument),
              );
            }
            break;
          case Document.ACTIONS.REPLACE:
          case Document.ACTIONS.DELETE: {
            if (!fetchedDocument) {
              result.addError(
                new DocumentNotFoundError(document),
              );

              break;
            }

            if (document.getRevision() !== fetchedDocument.getRevision() + 1) {
              result.addError(
                new InvalidDocumentRevisionError(document, fetchedDocument),
              );
            }

            break;
          }
          default:
            throw new InvalidDocumentActionError(document);
        }
      });

    if (!result.isValid()) {
      return result;
    }

    // Validate unique indices
    const nonDeleteActionDocuments = documents
      .filter((d) => d.getAction() !== Document.ACTIONS.DELETE);

    if (nonDeleteActionDocuments.length > 0) {
      result.merge(
        await validateDocumentsUniquenessByIndices(nonDeleteActionDocuments, dataContract),
      );

      if (!result.isValid()) {
        return result;
      }
    }

    // Run Data Triggers
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      dataProvider,
      userId,
      dataContract,
    );

    const dataTriggersExecutionResults = await executeDataTriggers(
      documents,
      dataTriggersExecutionContext,
    );

    dataTriggersExecutionResults.forEach((dataTriggerExecutionResult) => {
      if (!dataTriggerExecutionResult.isOk()) {
        result.addError(...dataTriggerExecutionResult.getErrors());
      }
    });

    return result;
  }

  return validateDocumentsSTData;
}

module.exports = validateDocumentsSTDataFactory;

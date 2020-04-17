const DataTriggerExecutionContext = require('../../../../dataTrigger/DataTriggerExecutionContext');

const ValidationResult = require('../../../../validation/ValidationResult');

const DocumentAlreadyPresentError = require('../../../../errors/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('../../../../errors/DocumentNotFoundError');
const DocumentOwnerIdMismatchError = require('../../../../errors/DocumentOwnerIdMismatchError');
const InvalidDocumentRevisionError = require('../../../../errors/InvalidDocumentRevisionError');
const InvalidDocumentActionError = require('../../../errors/InvalidDocumentActionError');
const DataContractNotPresentError = require('../../../../errors/DataContractNotPresentError');

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

/**
 *
 * @param {StateRepository} stateRepository
 * @param {fetchDocuments} fetchDocuments
 * @param {validateDocumentsUniquenessByIndices} validateDocumentsUniquenessByIndices
 * @param {executeDataTriggers} executeDataTriggers
 * @return {validateDocumentsBatchTransitionData}
 */
function validateDocumentsBatchTransitionDataFactory(
  stateRepository,
  fetchDocuments,
  validateDocumentsUniquenessByIndices,
  executeDataTriggers,
) {
  /**
   * @typedef validateDocumentsBatchTransitionData
   * @param {DocumentsBatchTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionData(stateTransition) {
    const result = new ValidationResult();

    const ownerId = stateTransition.getOwnerId();

    // Data contract must exist
    const dataContractId = stateTransition.getTransitions()[0].getDataContractId();
    const dataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    // Validate document action, ownerId and revision
    const documentTransitions = stateTransition.getTransitions();
    const fetchedDocuments = await fetchDocuments(dataContractId, documentTransitions);

    documentTransitions
      .forEach((documentTransition) => {
        const fetchedDocument = fetchedDocuments
          .find((d) => documentTransition.getId() === d.getId());

        switch (documentTransition.getAction()) {
          case AbstractDocumentTransition.ACTIONS.CREATE:
            if (fetchedDocument) {
              result.addError(
                new DocumentAlreadyPresentError(documentTransition, fetchedDocument),
              );
            }
            break;
          case AbstractDocumentTransition.ACTIONS.REPLACE: {
            if (
              fetchedDocument
              && documentTransition.getRevision() !== fetchedDocument.getRevision() + 1
            ) {
              result.addError(
                new InvalidDocumentRevisionError(documentTransition, fetchedDocument),
              );
            }
          }
          // eslint-disable-next-line no-fallthrough
          case AbstractDocumentTransition.ACTIONS.DELETE: {
            if (!fetchedDocument) {
              result.addError(
                new DocumentNotFoundError(documentTransition),
              );

              break;
            }

            if (fetchedDocument.getOwnerId() !== ownerId) {
              result.addError(
                new DocumentOwnerIdMismatchError(documentTransition, fetchedDocument),
              );
            }

            break;
          }
          default:
            throw new InvalidDocumentActionError(documentTransition);
        }
      });

    if (!result.isValid()) {
      return result;
    }

    // Validate unique indices
    const nonDeleteDocumentTransitions = documentTransitions
      .filter((d) => d.getAction() !== AbstractDocumentTransition.ACTIONS.DELETE);

    if (nonDeleteDocumentTransitions.length > 0) {
      result.merge(
        await validateDocumentsUniquenessByIndices(
          ownerId,
          nonDeleteDocumentTransitions,
          dataContract,
        ),
      );

      if (!result.isValid()) {
        return result;
      }
    }

    // Run Data Triggers
    const dataTriggersExecutionContext = new DataTriggerExecutionContext(
      stateRepository,
      ownerId,
      dataContract,
    );

    const dataTriggersExecutionResults = await executeDataTriggers(
      documentTransitions,
      dataTriggersExecutionContext,
    );

    dataTriggersExecutionResults.forEach((dataTriggerExecutionResult) => {
      if (!dataTriggerExecutionResult.isOk()) {
        result.addError(...dataTriggerExecutionResult.getErrors());
      }
    });

    return result;
  }

  return validateDocumentsBatchTransitionData;
}

module.exports = validateDocumentsBatchTransitionDataFactory;

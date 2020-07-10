const ValidationResult = require('../../../../validation/ValidationResult');

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

const DataContractNotPresentError = require('../../../../errors/DataContractNotPresentError');
const InvalidDocumentTransitionIdError = require('../../../../errors/InvalidDocumentTransitionIdError');
const InvalidDocumentTransitionEntropyError = require('../../../../errors/InvalidDocumentTransitionEntropyError');
const DuplicateDocumentTransitionsError = require('../../../../errors/DuplicateDocumentTransitionsError');
const MissingDocumentTypeError = require('../../../../errors/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../../errors/InvalidDocumentTypeError');
const InvalidDocumentTransitionActionError = require('../../../../errors/InvalidDocumentTransitionActionError');
const MissingDocumentTransitionActionError = require('../../../../errors/MissingDocumentTransitionActionError');
const MissingDataContractIdError = require('../../../../errors/MissingDataContractIdError');
const InvalidDataContractIdError = require('../../../../errors/InvalidDataContractIdError');

const DocumentsBatchTransition = require('../../DocumentsBatchTransition');

const baseTransitionSchema = require('../../../../../schema/document/stateTransition/documentTransition/base');
const createTransitionSchema = require('../../../../../schema/document/stateTransition/documentTransition/create');
const replaceTransitionSchema = require('../../../../../schema/document/stateTransition/documentTransition/replace');

const generateDocumentId = require('../../../generateDocumentId');
const entropy = require('../../../../util/entropy');

/**
 * @param {findDuplicatesById} findDuplicatesById
 * @param {findDuplicatesByIndices} findDuplicatesByIndices
 * @param {validateStateTransitionSignature} validateStateTransitionSignature
 * @param {validateIdentityExistence} validateIdentityExistence
 * @param {StateRepository} stateRepository
 * @param {JsonSchemaValidator} validator
 * @param {enrichDataContractWithBaseSchema} enrichDataContractWithBaseSchema
 *
 * @return {validateDocumentsBatchTransitionStructure}
 */
function validateDocumentsBatchTransitionStructureFactory(
  findDuplicatesById,
  findDuplicatesByIndices,
  validateStateTransitionSignature,
  validateIdentityExistence,
  stateRepository,
  validator,
  enrichDataContractWithBaseSchema,
) {
  const { ACTIONS } = AbstractDocumentTransition;

  async function validateDocumentTransitions(dataContractId, ownerId, documentTransitions) {
    const result = new ValidationResult();

    const dataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const enrichedBaseDataContract = enrichDataContractWithBaseSchema(
      dataContract,
      baseTransitionSchema,
    );

    const enrichedDataContractsByActions = {
      [ACTIONS.CREATE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        createTransitionSchema,
        'document_create_transition_',
      ),
      [ACTIONS.REPLACE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        replaceTransitionSchema,
        'document_replace_transition_',
        ['$createdAt'],
      ),
    };

    documentTransitions.forEach((rawDocumentTransition) => {
      // Validate $type
      if (!Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$type')) {
        result.addError(
          new MissingDocumentTypeError(rawDocumentTransition),
        );

        return;
      }

      if (!dataContract.isDocumentDefined(rawDocumentTransition.$type)) {
        result.addError(
          new InvalidDocumentTypeError(rawDocumentTransition.$type, dataContract),
        );

        return;
      }

      // Validate $action
      if (!Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$action')) {
        result.addError(
          new MissingDocumentTransitionActionError(rawDocumentTransition),
        );

        return;
      }

      // Validate document schema
      switch (rawDocumentTransition.$action) {
        case ACTIONS.CREATE:
        case ACTIONS.REPLACE: {
          // eslint-disable-next-line max-len
          const enrichedDataContract = enrichedDataContractsByActions[rawDocumentTransition.$action];

          const documentSchemaRef = enrichedDataContract.getDocumentSchemaRef(
            rawDocumentTransition.$type,
          );

          const additionalSchemas = {
            [enrichedDataContract.getJsonSchemaId()]:
            enrichedDataContract.toJSON(),
          };

          const schemaResult = validator.validate(
            documentSchemaRef,
            rawDocumentTransition,
            additionalSchemas,
          );

          if (!schemaResult.isValid()) {
            result.merge(schemaResult);

            break;
          }

          // Additional checks for CREATE transitions
          if (ACTIONS.CREATE === rawDocumentTransition.$action) {
            // validate id generation
            const documentId = generateDocumentId(
              dataContractId,
              ownerId,
              rawDocumentTransition.$type,
              rawDocumentTransition.$entropy,
            );

            if (rawDocumentTransition.$id !== documentId) {
              result.addError(
                new InvalidDocumentTransitionIdError(rawDocumentTransition),
              );
            }

            // validate entropy
            if (!entropy.validate(rawDocumentTransition.$entropy)) {
              result.addError(
                new InvalidDocumentTransitionEntropyError(rawDocumentTransition),
              );
            }
          }

          break;
        }
        case ACTIONS.DELETE:
          result.merge(
            validator.validate(
              baseTransitionSchema,
              rawDocumentTransition,
            ),
          );

          break;
        default:
          result.addError(
            new InvalidDocumentTransitionActionError(
              rawDocumentTransition.$action,
              rawDocumentTransition,
            ),
          );
      }
    });

    if (!result.isValid()) {
      return result;
    }

    // Find duplicate documents by type and ID
    const duplicateTransitions = findDuplicatesById(documentTransitions);
    if (duplicateTransitions.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitions),
      );
    }

    // Find duplicate transitions by unique indices
    const duplicateTransitionsByIndices = findDuplicatesByIndices(
      documentTransitions,
      dataContract,
    );

    if (duplicateTransitionsByIndices.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitionsByIndices),
      );
    }

    return result;
  }

  /**
   * @typedef validateDocumentsBatchTransitionStructure
   * @param {RawDocumentsBatchTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionStructure(rawStateTransition) {
    const result = new ValidationResult();

    const { ownerId } = rawStateTransition;

    // Group document transitions by data contracts
    const documentTransitionsByContracts = rawStateTransition.transitions
      .reduce((obj, rawDocumentTransition) => {
        if (!Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$dataContractId')) {
          result.addError(
            new MissingDataContractIdError(rawDocumentTransition),
          );

          return obj;
        }

        if (typeof rawDocumentTransition.$dataContractId !== 'string') {
          result.addError(
            new InvalidDataContractIdError(rawDocumentTransition.$dataContractId),
          );

          return obj;
        }

        if (!obj[rawDocumentTransition.$dataContractId]) {
          // eslint-disable-next-line no-param-reassign
          obj[rawDocumentTransition.$dataContractId] = [];
        }

        obj[rawDocumentTransition.$dataContractId].push(rawDocumentTransition);

        return obj;
      }, {});

    const documentTransitionResultsPromises = Object.entries(documentTransitionsByContracts)
      .map(([dataContractId, documentTransitions]) => (
        validateDocumentTransitions(dataContractId, ownerId, documentTransitions)
      ));

    const documentTransitionResults = await Promise.all(documentTransitionResultsPromises);
    documentTransitionResults.forEach(result.merge.bind(result));

    if (!result.isValid()) {
      return result;
    }

    // User must exist and confirmed
    result.merge(
      await validateIdentityExistence(
        ownerId,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    const stateTransition = new DocumentsBatchTransition(rawStateTransition);

    // Verify ST signature
    result.merge(
      await validateStateTransitionSignature(stateTransition, ownerId),
    );

    return result;
  }

  return validateDocumentsBatchTransitionStructure;
}

module.exports = validateDocumentsBatchTransitionStructureFactory;

const ValidationResult = require('../../../../validation/ValidationResult');

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

const DataContractNotPresentError = require('../../../../errors/DataContractNotPresentError');
const InvalidDocumentTransitionIdError = require('../../../../errors/InvalidDocumentTransitionIdError');
const InvalidDocumentTransitionEntropyError = require('../../../../errors/InvalidDocumentTransitionEntropyError');
const DuplicateDocumentTransitionsError = require('../../../../errors/DuplicateDocumentTransitionsError');

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
  /**
   * @typedef validateDocumentsBatchTransitionStructure
   * @param {RawDocumentsBatchTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionStructure(rawStateTransition) {
    const result = new ValidationResult();

    const {
      ownerId,
      transitions: [{ $dataContractId: dataContractId }],
    } = rawStateTransition;

    // check data contract exists
    const dataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const createTransitions = rawStateTransition.transitions
      .filter(
        (t) => (t.$action === AbstractDocumentTransition.ACTIONS.CREATE),
      );

    const replaceTransitions = rawStateTransition.transitions
      .filter(
        (t) => (t.$action === AbstractDocumentTransition.ACTIONS.REPLACE),
      );

    const deleteTransitions = rawStateTransition.transitions
      .filter(
        (t) => (t.$action === AbstractDocumentTransition.ACTIONS.DELETE),
      );

    const enrichedBaseDataContract = enrichDataContractWithBaseSchema(
      dataContract,
      baseTransitionSchema,
    );

    const enrichedDataContractsByActions = {
      [AbstractDocumentTransition.ACTIONS.CREATE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        createTransitionSchema,
      ),
      [AbstractDocumentTransition.ACTIONS.REPLACE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        replaceTransitionSchema,
      ),
    };

    // Validate schema of CREATE and REPLACE transitions
    createTransitions
      .concat(replaceTransitions)
      .forEach((rawTransition) => {
        // validate document schema
        const documentSchemaRef = dataContract.getDocumentSchemaRef(rawTransition.$type);

        const additionalSchemas = {
          [dataContract.getJsonSchemaId()]: enrichedDataContractsByActions[rawTransition.$action],
        };

        result.merge(
          validator.validate(
            documentSchemaRef,
            rawTransition,
            additionalSchemas,
          ),
        );
      });

    // validate schema of DELETE transitions
    deleteTransitions
      .forEach((rawTransition) => {
        result.merge(
          validator.validate(
            baseTransitionSchema,
            rawTransition,
          ),
        );
      });

    if (!result.isValid()) {
      return result;
    }

    // additional checks
    createTransitions
      .forEach((rawTransition) => {
        // validate id generation
        const documentId = generateDocumentId(
          dataContractId,
          ownerId,
          rawTransition.$type,
          rawTransition.$entropy,
        );

        if (rawTransition.$id !== documentId) {
          result.addError(
            new InvalidDocumentTransitionIdError(rawTransition),
          );
        }

        // validate entropy
        if (!entropy.validate(rawTransition.$entropy)) {
          result.addError(
            new InvalidDocumentTransitionEntropyError(rawTransition),
          );
        }
      });

    // Find duplicate documents by type and ID
    const duplicateTransitions = findDuplicatesById(rawStateTransition.transitions);
    if (duplicateTransitions.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitions),
      );
    }

    // Find duplicate transitions by unique indices
    const duplicateTransitionsByIndices = findDuplicatesByIndices(
      rawStateTransition.transitions,
      dataContract,
    );

    if (duplicateTransitionsByIndices.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitionsByIndices),
      );
    }

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

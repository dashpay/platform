const ValidationResult = require('../../../../../validation/ValidationResult');

const AbstractDocumentTransition = require('../../documentTransition/AbstractDocumentTransition');

const DataContractNotPresentError = require('../../../../../errors/consensus/basic/document/DataContractNotPresentError');
const InvalidDocumentTransitionIdError = require('../../../../../errors/consensus/basic/document/InvalidDocumentTransitionIdError');
const DuplicateDocumentTransitionsError = require('../../../../../errors/consensus/basic/document/DuplicateDocumentTransitionsError');
const MissingDocumentTransitionTypeError = require('../../../../../errors/consensus/basic/document/MissingDocumentTransitionTypeError');
const InvalidDocumentTypeError = require('../../../../../errors/consensus/basic/document/InvalidDocumentTypeError');
const InvalidDocumentTransitionActionError = require('../../../../../errors/consensus/basic/document/InvalidDocumentTransitionActionError');
const MissingDocumentTransitionActionError = require('../../../../../errors/consensus/basic/document/MissingDocumentTransitionActionError');
const MissingDataContractIdError = require('../../../../../errors/consensus/basic/document/MissingDataContractIdError');
const Identifier = require('../../../../../identifier/Identifier');

const baseTransitionSchema = require('../../../../../../schema/document/stateTransition/documentTransition/base.json');
const createTransitionSchema = require('../../../../../../schema/document/stateTransition/documentTransition/create.json');
const replaceTransitionSchema = require('../../../../../../schema/document/stateTransition/documentTransition/replace.json');

const generateDocumentId = require('../../../../generateDocumentId');
const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

const documentsBatchTransitionSchema = require('../../../../../../schema/document/stateTransition/documentsBatch.json');
const createAndValidateIdentifier = require('../../../../../identifier/createAndValidateIdentifier');

/**
 * @param {findDuplicatesById} findDuplicatesById
 * @param {findDuplicatesByIndices} findDuplicatesByIndices
 * @param {StateRepository} stateRepository
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {enrichDataContractWithBaseSchema} enrichDataContractWithBaseSchema
 * @param {validatePartialCompoundIndices} validatePartialCompoundIndices
 *
 * @return {validateDocumentsBatchTransitionBasic}
 */
function validateDocumentsBatchTransitionBasicFactory(
  findDuplicatesById,
  findDuplicatesByIndices,
  stateRepository,
  jsonSchemaValidator,
  enrichDataContractWithBaseSchema,
  validatePartialCompoundIndices,
) {
  const { ACTIONS } = AbstractDocumentTransition;

  /**
   *
   * @param {DataContract} dataContract
   * @param {Buffer} ownerId
   * @param {Array<
   *      RawDocumentCreateTransition|
   *      RawDocumentDeleteTransition|
   *      DocumentReplaceTransition
   *      >} rawDocumentTransitions
   * @return {Promise<ValidationResult>}
   */
  async function validateDocumentTransitions(dataContract, ownerId, rawDocumentTransitions) {
    const result = new ValidationResult();

    if (!result.isValid()) {
      return result;
    }

    const enrichedBaseDataContract = enrichDataContractWithBaseSchema(
      dataContract,
      baseTransitionSchema,
      enrichDataContractWithBaseSchema.PREFIX_BYTE_1,
    );

    const enrichedDataContractsByActions = {
      [ACTIONS.CREATE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        createTransitionSchema,
        enrichDataContractWithBaseSchema.PREFIX_BYTE_2,
      ),
      [ACTIONS.REPLACE]: enrichDataContractWithBaseSchema(
        enrichedBaseDataContract,
        replaceTransitionSchema,
        enrichDataContractWithBaseSchema.PREFIX_BYTE_3,
        ['$createdAt'],
      ),
    };

    rawDocumentTransitions.forEach((rawDocumentTransition) => {
      // Validate $type
      if (!Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$type')) {
        result.addError(
          new MissingDocumentTransitionTypeError(rawDocumentTransition),
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

          const schemaResult = jsonSchemaValidator.validate(
            documentSchemaRef,
            convertBuffersToArrays(rawDocumentTransition),
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
              dataContract.getId(),
              ownerId,
              rawDocumentTransition.$type,
              rawDocumentTransition.$entropy,
            );

            if (!rawDocumentTransition.$id.equals(documentId)) {
              result.addError(
                new InvalidDocumentTransitionIdError(rawDocumentTransition),
              );
            }
          }

          break;
        }
        case ACTIONS.DELETE:
          result.merge(
            jsonSchemaValidator.validate(
              baseTransitionSchema,
              convertBuffersToArrays(rawDocumentTransition),
            ),
          );

          break;
        default:
          throw new InvalidDocumentTransitionActionError(
            rawDocumentTransition.$action,
            rawDocumentTransition,
          );
      }
    });

    if (!result.isValid()) {
      return result;
    }

    // Find duplicate documents by type and ID
    const duplicateTransitions = findDuplicatesById(rawDocumentTransitions);
    if (duplicateTransitions.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitions),
      );
    }

    // Find duplicate transitions by unique indices
    const duplicateTransitionsByIndices = findDuplicatesByIndices(
      rawDocumentTransitions,
      dataContract,
    );

    if (duplicateTransitionsByIndices.length > 0) {
      result.addError(
        new DuplicateDocumentTransitionsError(duplicateTransitionsByIndices),
      );
    }

    // Validate partial compound indices
    const nonDeleteDocumentTransitions = rawDocumentTransitions
      .filter((d) => d.$action !== AbstractDocumentTransition.ACTIONS.DELETE);

    if (nonDeleteDocumentTransitions.length > 0) {
      result.merge(
        validatePartialCompoundIndices(
          ownerId,
          nonDeleteDocumentTransitions,
          dataContract,
        ),
      );
    }

    return result;
  }

  /**
   * @typedef validateDocumentsBatchTransitionBasic
   * @param {RawDocumentsBatchTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDocumentsBatchTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      documentsBatchTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    // Group document transitions by data contracts
    const documentTransitionsByContracts = rawStateTransition.transitions
      .reduce((obj, rawDocumentTransition) => {
        if (!Object.prototype.hasOwnProperty.call(rawDocumentTransition, '$dataContractId')) {
          result.addError(
            new MissingDataContractIdError(rawDocumentTransition),
          );

          return obj;
        }

        const dataContractId = createAndValidateIdentifier(
          '$dataContractId',
          rawDocumentTransition.$dataContractId,
          result,
        );

        if (!dataContractId) {
          return obj;
        }

        if (!obj[dataContractId]) {
          // eslint-disable-next-line no-param-reassign
          obj[dataContractId] = [];
        }

        obj[dataContractId].push(rawDocumentTransition);

        return obj;
      }, {});

    const documentTransitionResultsPromises = Object.entries(documentTransitionsByContracts)
      .map(async ([dataContractIdString, documentTransitions]) => {
        const perDocumentResult = new ValidationResult();

        const dataContractId = Identifier.from(dataContractIdString);

        const dataContract = await stateRepository.fetchDataContract(dataContractId);

        if (!dataContract) {
          perDocumentResult.addError(
            new DataContractNotPresentError(dataContractId),
          );
        }

        if (!perDocumentResult.isValid()) {
          return perDocumentResult;
        }

        perDocumentResult.merge(
          await validateDocumentTransitions(
            dataContract,
            rawStateTransition.ownerId,
            documentTransitions,
          ),
        );

        return perDocumentResult;
      });

    const documentTransitionResults = await Promise.all(documentTransitionResultsPromises);
    documentTransitionResults.forEach(result.merge.bind(result));

    return result;
  }

  return validateDocumentsBatchTransitionBasic;
}

module.exports = validateDocumentsBatchTransitionBasicFactory;

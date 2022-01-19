const lodashClone = require('lodash.clonedeep');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

const dataContractUpdateTransitionSchema = require('../../../../../../schema/dataContract/stateTransition/dataContractUpdate.json');

const IncompatibleDataContractSchemaError = require('../../../../../errors/consensus/basic/dataContract/IncompatibleDataContractSchemaError');
const DataContractImmutablePropertiesUpdateError = require('../../../../../errors/consensus/basic/dataContract/DataContractImmutablePropertiesUpdateError');
const InvalidDataContractVersionError = require('../../../../../errors/consensus/basic/dataContract/InvalidDataContractVersionError');
const DataContractNotPresentError = require('../../../../../errors/consensus/basic/document/DataContractNotPresentError');

const Identifier = require('../../../../../identifier/Identifier');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContract} validateDataContract
 * @param {validateProtocolVersion} validateProtocolVersion
 * @param {StateRepository} stateRepository
 * @param {DiffValidator} diffValidator
 * @param {validateIndicesAreNotChanged} validateIndicesAreNotChanged
 * @param {JsonPatch} jsonPatch
 *
 * @return {validateDataContractUpdateTransitionBasic}
 */
function validateDataContractUpdateTransitionBasicFactory(
  jsonSchemaValidator,
  validateDataContract,
  validateProtocolVersion,
  stateRepository,
  diffValidator,
  validateIndicesAreNotChanged,
  jsonPatch,
) {
  /**
   * @typedef validateDataContractUpdateTransitionBasic
   * @param {RawDataContractUpdateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractUpdateTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      dataContractUpdateTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validateProtocolVersion(rawStateTransition.protocolVersion),
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate Data Contract
    const rawDataContract = rawStateTransition.dataContract;

    result.merge(
      await validateDataContract(rawDataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    const dataContractId = Identifier.from(rawDataContract.$id);

    // Data contract should exist
    const existingDataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!existingDataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId.toBuffer()),
      );

      return result;
    }

    // Version difference should be exactly 1
    const oldVersion = existingDataContract.getVersion();
    const newVersion = rawDataContract.version;
    const versionDiff = newVersion - oldVersion;

    if (versionDiff !== 1) {
      result.addError(
        new InvalidDataContractVersionError(
          oldVersion + 1,
          oldVersion + versionDiff,
        ),
      );
    }

    // check that only $defs, version and documents are changed
    const oldBaseDataContract = lodashClone(existingDataContract.toObject());
    delete oldBaseDataContract.$defs;
    delete oldBaseDataContract.documents;
    delete oldBaseDataContract.version;

    oldBaseDataContract.$id = oldBaseDataContract.$id.toString('hex');
    oldBaseDataContract.ownerId = oldBaseDataContract.ownerId.toString('hex');

    const newBaseDataContract = lodashClone(rawDataContract);
    delete newBaseDataContract.$defs;
    delete newBaseDataContract.documents;
    delete newBaseDataContract.version;

    newBaseDataContract.$id = newBaseDataContract.$id.toString('hex');
    newBaseDataContract.ownerId = newBaseDataContract.ownerId.toString('hex');

    const baseDataContractDiff = jsonPatch.compare(
      oldBaseDataContract,
      newBaseDataContract,
    );

    if (baseDataContractDiff.length > 0) {
      const { op: operation, path: fieldPath } = baseDataContractDiff[0];

      const error = new DataContractImmutablePropertiesUpdateError(operation, fieldPath);
      error.setDiff(baseDataContractDiff);

      result.addError(error);

      return result;
    }

    // check indices are not changed
    result.merge(
      validateIndicesAreNotChanged(
        existingDataContract.getDocuments(),
        rawDataContract.documents,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    // Schema should be backward compatible
    const oldSchema = existingDataContract.getDocuments();
    const newSchema = rawDataContract.documents;

    Object.entries(oldSchema)
      .forEach(([documentType, documentSchema]) => {
        try {
          diffValidator.validateSchemaCompatibility(
            documentSchema,
            newSchema[documentType] || {},
          );
        } catch (schemaValidationError) {
          const regexp = /change = (.*?)$/;

          const match = schemaValidationError.message.match(regexp);

          const validationErrorOperations = JSON.parse(match[1]);

          const { op: operation, path: fieldPath } = validationErrorOperations[0];

          const error = new IncompatibleDataContractSchemaError(
            existingDataContract.getId().toBuffer(),
            operation,
            fieldPath,
          );
          error.setOldSchema(documentSchema);
          error.setNewSchema(newSchema[documentType]);
          error.setValidationError(schemaValidationError);

          result.addError(error);
        }
      });

    return result;
  }

  return validateDataContractUpdateTransitionBasic;
}

/**
 * @typedef {Object} DiffValidator
 * @property {function(Object, Object)} validateSchemaCompatibility
 */

/**
 * @typedef {Object} JsonPatch
 * @property {function(Object, Object)} compare
 */

module.exports = validateDataContractUpdateTransitionBasicFactory;

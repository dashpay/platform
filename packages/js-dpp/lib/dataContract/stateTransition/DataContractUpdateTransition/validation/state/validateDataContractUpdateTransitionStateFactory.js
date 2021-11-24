const serializer = require('../../../../../util/serializer');

const ValidationResult = require('../../../../../validation/ValidationResult');

const IncompatibleDataContractSchemaError = require('../../../../../errors/consensus/state/dataContract/IncompatibleDataContractSchemaError');
const InvalidDataContractBaseDataError = require('../../../../../errors/consensus/state/dataContract/InvalidDataContractBaseDataError');
const InvalidDataContractVersionError = require('../../../../../errors/consensus/state/dataContract/InvalidDataContractVersionError');
const DataContractNotPresentError = require('../../../../../errors/consensus/basic/document/DataContractNotPresentError');

/**
 *
 * @param {StateRepository} stateRepository
 * @param {DiffValidator} diffValidator
 * @param {validateIndicesAreNotChanged} validateIndicesAreNotChanged
 * @return {validateDataContractUpdateTransitionState}
 */
function validateDataContractUpdateTransitionStateFactory(
  stateRepository,
  diffValidator,
  validateIndicesAreNotChanged,
) {
  /**
   * @typedef validateDataContractUpdateTransitionState
   * @param {DataContractCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractUpdateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data contract should exist
    const existingDataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!existingDataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId.toBuffer()),
      );
    }

    // Version difference should be exactly 1
    const oldVersion = existingDataContract.getVersion();
    const newVersion = stateTransition.getDataContract().getVersion();
    const versionDiff = newVersion - oldVersion;

    if (versionDiff !== 1) {
      result.addError(
        new InvalidDataContractVersionError(
          oldVersion + 1,
          oldVersion + versionDiff,
        ),
      );
    }

    // Schema should be backward compatible
    const oldSchema = existingDataContract.getDocuments();
    const newSchema = stateTransition.getDataContract().getDocuments();

    try {
      diffValidator.validateSchemaCompatibility(oldSchema, newSchema);
    } catch (e) {
      result.addError(new IncompatibleDataContractSchemaError(
        oldSchema,
        newSchema,
        e,
        existingDataContract.getId(),
      ));
    }

    // check that only $defs, $version and documents are changed
    const oldBaseDataContract = existingDataContract.toObject();
    delete oldBaseDataContract.$defs;
    delete oldBaseDataContract.documents;
    delete oldBaseDataContract.$version;

    const newBaseDataContract = stateTransition.getDataContract().toObject();
    delete newBaseDataContract.$defs;
    delete newBaseDataContract.documents;
    delete newBaseDataContract.$version;

    if (!serializer.encode(oldBaseDataContract).equals(serializer.encode(newBaseDataContract))) {
      result.addError(
        new InvalidDataContractBaseDataError(oldBaseDataContract, newBaseDataContract),
      );
    }

    // check indices are not changed
    result.merge(
      await validateIndicesAreNotChanged(
        existingDataContract.getDocuments(),
        stateTransition.getDataContract().getDocuments(),
      ),
    );

    return result;
  }

  return validateDataContractUpdateTransitionState;
}

/**
 * @typedef {Object} DiffValidator
 * @property {function(Object, Object)} validateSchemaCompatibility
 */

module.exports = validateDataContractUpdateTransitionStateFactory;

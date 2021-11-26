const lodashClone = require('lodash.clonedeep');

const serializer = require('../../../../../util/serializer');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

const dataContractUpdateTransitionSchema = require('../../../../../../schema/dataContract/stateTransition/dataContractUpdate.json');

const IncompatibleDataContractSchemaError = require('../../../../../errors/consensus/basic/dataContract/IncompatibleDataContractSchemaError');
const InvalidDataContractBaseDataError = require('../../../../../errors/consensus/basic/dataContract/InvalidDataContractBaseDataError');
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

    // Schema should be backward compatible
    const oldSchema = existingDataContract.getDocuments();
    const newSchema = rawDataContract.documents;

    try {
      diffValidator.validateSchemaCompatibility(oldSchema, newSchema);
    } catch (e) {
      result.addError(new IncompatibleDataContractSchemaError(
        oldSchema,
        newSchema,
        e,
        existingDataContract.getId(),
      ));

      return result;
    }

    // check that only $defs, version and documents are changed
    const oldBaseDataContract = existingDataContract.toObject();
    delete oldBaseDataContract.$defs;
    delete oldBaseDataContract.documents;
    delete oldBaseDataContract.version;

    const newBaseDataContract = lodashClone(rawDataContract);
    delete newBaseDataContract.$defs;
    delete newBaseDataContract.documents;
    delete newBaseDataContract.version;

    if (!serializer.encode(oldBaseDataContract).equals(serializer.encode(newBaseDataContract))) {
      result.addError(
        new InvalidDataContractBaseDataError(oldBaseDataContract, newBaseDataContract),
      );

      return result;
    }

    // check indices are not changed
    result.merge(
      await validateIndicesAreNotChanged(
        existingDataContract.getDocuments(),
        rawDataContract.documents,
      ),
    );

    return result;
  }

  return validateDataContractUpdateTransitionBasic;
}

module.exports = validateDataContractUpdateTransitionBasicFactory;

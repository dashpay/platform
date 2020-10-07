const $RefParser = require('@apidevtools/json-schema-ref-parser');
const { Transaction } = require('@dashevo/dashcore-lib');

const MissingOptionError = require('../errors/MissingOptionError');

const StateTransitionFactory = require('./StateTransitionFactory');

const AbstractStateTransition = require('./AbstractStateTransition');

const stateTransitionTypes = require('./stateTransitionTypes');

const createStateTransitionFactory = require('./createStateTransitionFactory');

const validateDataContractFactory = require('../dataContract/validateDataContractFactory');
const validateDataContractCreateTransitionStructureFactory = require('../dataContract/stateTransition/validation/validateDataContractCreateTransitionStructureFactory');
const validateStateTransitionStructureFactory = require('./validation/validateStateTransitionStructureFactory');
const validateDataContractCreateTransitionDataFactory = require('../dataContract/stateTransition/validation/validateDataContractCreateTransitionDataFactory');
const validateStateTransitionDataFactory = require('./validation/validateStateTransitionDataFactory');
const validateDocumentsBatchTransitionStructureFactory = require('../document/stateTransition/validation/structure/validateDocumentsBatchTransitionStructureFactory');
const validateIdentityCreateTransitionDataFactory = require('../identity/stateTransitions/identityCreateTransition/validateIdentityCreateTransitionDataFactory');
const validateIdentityTopUpTransitionDataFactory = require('../identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionDataFactory');
const validateAssetLockTransactionFactory = require('../identity/stateTransitions/identityCreateTransition/validateAssetLockTransactionFactory');
const validateIdentityCreateTransitionStructureFactory = require('../identity/stateTransitions/identityCreateTransition/validateIdentityCreateTransitionStructureFactory');
const validateIdentityTopUpTransitionStructureFactory = require('../identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionStructureFactory');
const validateIdentityPublicKeysUniquenessFactory = require('../identity/stateTransitions/identityCreateTransition/validateIdentityPublicKeysUniquenessFactory');
const validateStateTransitionSignatureFactory = require('./validation/validateStateTransitionSignatureFactory');
const validateStateTransitionFeeFactory = require('./validation/validateStateTransitionFeeFactory');
const fetchConfirmedAssetLockTransactionOutputFactory = require('./fetchConfirmedAssetLockTransactionOutputFactory');

const enrichDataContractWithBaseSchema = require('../dataContract/enrichDataContractWithBaseSchema');
const findDuplicatesById = require('../document/stateTransition/validation/structure/findDuplicatesById');
const findDuplicatesByIndices = require('../document/stateTransition/validation/structure/findDuplicatesByIndices');

const validateDocumentsBatchTransitionDataFactory = require('../document/stateTransition/validation/data/validateDocumentsBatchTransitionDataFactory');
const fetchDocumentsFactory = require('../document/stateTransition/validation/data/fetchDocumentsFactory');
const validateDocumentsUniquenessByIndicesFactory = require('../document/stateTransition/validation/data/validateDocumentsUniquenessByIndicesFactory');
const validatePartialCompoundIndices = require('../document/stateTransition/validation/data/validatePartialCompoundIndices');
const getDataTriggersFactory = require('../dataTrigger/getDataTriggersFactory');
const executeDataTriggersFactory = require('../document/stateTransition/validation/data/executeDataTriggersFactory');
const validateIdentityExistenceFactory = require('./validation/validateIdentityExistenceFactory');
const validatePublicKeysFactory = require('../identity/validation/validatePublicKeysFactory');
const validateDataContractMaxDepthFactory = require('../dataContract/stateTransition/validation/validateDataContractMaxDepthFactory');

const applyStateTransitionFactory = require('./applyStateTransitionFactory');

const applyDataContractCreateTransitionFactory = require(
  '../dataContract/stateTransition/applyDataContractCreateTransitionFactory',
);

const applyDocumentsBatchTransitionFactory = require(
  '../document/stateTransition/applyDocumentsBatchTransitionFactory',
);

const applyIdentityCreateTransitionFactory = require(
  '../identity/stateTransitions/identityCreateTransition/applyIdentityCreateTransitionFactory',
);

const applyIdentityTopUpTransitionFactory = require(
  '../identity/stateTransitions/identityTopUpTransition/applyIdentityTopUpTransitionFactory',
);

class StateTransitionFacade {
  /**
   * @param {StateRepository} stateRepository
   * @param {JsonSchemaValidator} validator
   * @param {boolean} [skipAssetLockConfirmationValidation=false]
   */
  constructor(stateRepository, validator, skipAssetLockConfirmationValidation = false) {
    this.stateRepository = stateRepository;
    this.validator = validator;

    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    const validateDataContract = validateDataContractFactory(
      validator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
    );

    const validateStateTransitionSignature = validateStateTransitionSignatureFactory(
      stateRepository,
    );

    const validateIdentityExistence = validateIdentityExistenceFactory(stateRepository);

    // eslint-disable-next-line max-len
    const validateDataContractCreateTransitionStructure = validateDataContractCreateTransitionStructureFactory(
      validator,
      validateDataContract,
      validateStateTransitionSignature,
      validateIdentityExistence,
    );

    this.createStateTransition = createStateTransitionFactory(stateRepository);

    const validateDocumentsBatchTransitionStructure = (
      validateDocumentsBatchTransitionStructureFactory(
        findDuplicatesById,
        findDuplicatesByIndices,
        validateStateTransitionSignature,
        validateIdentityExistence,
        stateRepository,
        validator,
        enrichDataContractWithBaseSchema,
      )
    );

    const validatePublicKeys = validatePublicKeysFactory(
      validator,
    );

    const validateIdentityCreateTransitionStructure = (
      validateIdentityCreateTransitionStructureFactory(
        validator,
        validatePublicKeys,
      )
    );

    const validateIdentityTopUpTransitionStructure = (
      validateIdentityTopUpTransitionStructureFactory(
        validator,
      )
    );

    const validationFunctionsByType = {
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractCreateTransitionStructure,
      [stateTransitionTypes.DOCUMENTS_BATCH]: validateDocumentsBatchTransitionStructure,
      [stateTransitionTypes.IDENTITY_CREATE]: validateIdentityCreateTransitionStructure,
      [stateTransitionTypes.IDENTITY_TOP_UP]: validateIdentityTopUpTransitionStructure,
    };

    this.validateStateTransitionStructure = validateStateTransitionStructureFactory(
      validationFunctionsByType,
      this.createStateTransition,
    );

    const validateDataContractCreateTransitionData = (
      validateDataContractCreateTransitionDataFactory(
        stateRepository,
      )
    );

    // eslint-disable-next-line max-len
    const fetchConfirmedAssetLockTransactionOutput = fetchConfirmedAssetLockTransactionOutputFactory(
      stateRepository,
      Transaction.parseOutPointBuffer,
      skipAssetLockConfirmationValidation,
    );

    const validateAssetLockTransaction = validateAssetLockTransactionFactory(
      fetchConfirmedAssetLockTransactionOutput,
    );

    const validateIdentityPublicKeysUniqueness = validateIdentityPublicKeysUniquenessFactory(
      stateRepository,
    );

    const validateIdentityCreateTransitionData = validateIdentityCreateTransitionDataFactory(
      stateRepository,
      validateAssetLockTransaction,
      validateIdentityPublicKeysUniqueness,
    );

    const validateIdentityTopUpTransitionData = validateIdentityTopUpTransitionDataFactory(
      validateAssetLockTransaction,
      validateIdentityExistence,
    );

    const fetchDocuments = fetchDocumentsFactory(
      stateRepository,
    );

    const validateDocumentsUniquenessByIndices = validateDocumentsUniquenessByIndicesFactory(
      stateRepository,
    );

    const getDataTriggers = getDataTriggersFactory();

    const executeDataTriggers = executeDataTriggersFactory(
      getDataTriggers,
    );

    const validateDocumentsBatchTransitionData = validateDocumentsBatchTransitionDataFactory(
      stateRepository,
      fetchDocuments,
      validateDocumentsUniquenessByIndices,
      validatePartialCompoundIndices,
      executeDataTriggers,
    );

    this.validateStateTransitionData = validateStateTransitionDataFactory({
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractCreateTransitionData,
      [stateTransitionTypes.DOCUMENTS_BATCH]: validateDocumentsBatchTransitionData,
      [stateTransitionTypes.IDENTITY_CREATE]: validateIdentityCreateTransitionData,
      [stateTransitionTypes.IDENTITY_TOP_UP]: validateIdentityTopUpTransitionData,
    });

    this.validateStateTransitionFee = validateStateTransitionFeeFactory(
      stateRepository,
      fetchConfirmedAssetLockTransactionOutput,
    );

    this.factory = new StateTransitionFactory(
      this.validateStateTransitionStructure,
      this.createStateTransition,
    );

    const applyDataContractCreateTransition = applyDataContractCreateTransitionFactory(
      stateRepository,
    );

    const applyDocumentsBatchTransition = applyDocumentsBatchTransitionFactory(
      stateRepository,
      fetchDocuments,
    );

    const applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      stateRepository,
      fetchConfirmedAssetLockTransactionOutput,
    );

    const applyIdentityTopUpTransition = applyIdentityTopUpTransitionFactory(
      stateRepository,
      fetchConfirmedAssetLockTransactionOutput,
    );

    this.applyStateTransition = applyStateTransitionFactory(
      applyDataContractCreateTransition,
      applyDocumentsBatchTransition,
      applyIdentityCreateTransition,
      applyIdentityTopUpTransition,
    );
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromObject(rawStateTransition, options = {}) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.factory.createFromObject(rawStateTransition, options);
  }

  /**
   * Create State Transition from buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromBuffer(buffer, options = {}) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.factory.createFromBuffer(buffer, options);
  }

  /**
   * Validate State Transition
   *
   * @param {RawStateTransition|AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async validate(stateTransition) {
    const result = await this.validateStructure(stateTransition);

    if (!result.isValid()) {
      return result;
    }

    let stateTransitionModel = stateTransition;

    if (!(stateTransition instanceof AbstractStateTransition)) {
      stateTransitionModel = this.createStateTransition(stateTransition);
    }

    return this.validateData(stateTransitionModel);
  }

  /**
   * Validate State Transition Structure
   *
   * @param {AbstractStateTransition|RawStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async validateStructure(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    let rawStateTransition;
    if (stateTransition instanceof AbstractStateTransition) {
      rawStateTransition = stateTransition.toObject();
    } else {
      rawStateTransition = stateTransition;
    }

    return this.validateStateTransitionStructure(rawStateTransition);
  }

  /**
   * Validate State Transition Data
   *
   * @param {AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async validateData(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.validateStateTransitionData(stateTransition);
  }

  /**
   * Validate State Transition Fee
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @return {ValidationResult}
   */
  async validateFee(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.validateStateTransitionFee(stateTransition);
  }

  /**
   * Apply state transition to the state
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async apply(stateTransition) {
    return this.applyStateTransition(stateTransition);
  }
}

module.exports = StateTransitionFacade;

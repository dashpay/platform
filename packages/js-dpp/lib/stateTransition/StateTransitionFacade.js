const $RefParser = require('@apidevtools/json-schema-ref-parser');
const { Transaction } = require('@dashevo/dashcore-lib');

const MissingOptionError = require('../errors/MissingOptionError');

const StateTransitionFactory = require('./StateTransitionFactory');

const AbstractStateTransition = require('./AbstractStateTransition');

const stateTransitionTypes = require('./stateTransitionTypes');

const dataContractCreateTransitionSchema = require('../../schema/dataContract/stateTransition/dataContractCreate');
const documentsBatchTransitionSchema = require('../../schema/document/stateTransition/documentsBatch');
const identityCreateTransitionSchema = require('../../schema/identity/stateTransition/identityCreate');
const identityTopUpTransitionSchema = require('../../schema/identity/stateTransition/identityTopUp');

const createStateTransitionFactory = require('./createStateTransitionFactory');

const validateDataContractFactory = require('../dataContract/validateDataContractFactory');
const validateDataContractCreateTransitionStructureFactory = require('../dataContract/stateTransition/validation/validateDataContractCreateTransitionStructureFactory');
const validateStateTransitionStructureFactory = require('./validation/validateStateTransitionStructureFactory');
const validateDataContractCreateTransitionDataFactory = require('../dataContract/stateTransition/validation/validateDataContractCreateTransitionDataFactory');
const validateStateTransitionDataFactory = require('./validation/validateStateTransitionDataFactory');
const validateDocumentsBatchTransitionStructureFactory = require('../document/stateTransition/validation/structure/validateDocumentsBatchTransitionStructureFactory');
const validateIdentityCreateSTDataFactory = require('../identity/stateTransitions/identityCreateTransition/validateIdentityCreateSTDataFactory');
const validateIdentityTopUpTransitionDataFactory = require('../identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionDataFactory');
const validateAssetLockTransactionFactory = require('../identity/stateTransitions/identityCreateTransition/validateAssetLockTransactionFactory');
const validateIdentityCreateSTStructureFactory = require('../identity/stateTransitions/identityCreateTransition/validateIdentityCreateSTStructureFactory');
const validateIdentityTopUpTransitionStructure = require('../identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionStructure');
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

    const validateIdentityCreateSTStructure = validateIdentityCreateSTStructureFactory(
      validatePublicKeys,
    );

    const typeExtensions = {
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: {
        validationFunction: validateDataContractCreateTransitionStructure,
        schema: dataContractCreateTransitionSchema,
      },
      [stateTransitionTypes.DOCUMENTS_BATCH]: {
        validationFunction: validateDocumentsBatchTransitionStructure,
        schema: documentsBatchTransitionSchema,
      },
      [stateTransitionTypes.IDENTITY_CREATE]: {
        validationFunction: validateIdentityCreateSTStructure,
        schema: identityCreateTransitionSchema,
      },
      [stateTransitionTypes.IDENTITY_TOP_UP]: {
        validationFunction: validateIdentityTopUpTransitionStructure,
        schema: identityTopUpTransitionSchema,
      },
    };

    this.validateStateTransitionStructure = validateStateTransitionStructureFactory(
      validator,
      typeExtensions,
      this.createStateTransition,
    );

    const
      validateDataContractCreateTransitionData = validateDataContractCreateTransitionDataFactory(
        stateRepository,
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

    const validateIdentityCreateSTData = validateIdentityCreateSTDataFactory(
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

    const validateDocumentsSTData = validateDocumentsBatchTransitionDataFactory(
      stateRepository,
      fetchDocuments,
      validateDocumentsUniquenessByIndices,
      validatePartialCompoundIndices,
      executeDataTriggers,
    );

    this.validateStateTransitionData = validateStateTransitionDataFactory({
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractCreateTransitionData,
      [stateTransitionTypes.DOCUMENTS_BATCH]: validateDocumentsSTData,
      [stateTransitionTypes.IDENTITY_CREATE]: validateIdentityCreateSTData,
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
   * Create State Transition from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromSerialized(payload, options = {}) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.factory.createFromSerialized(payload, options);
  }

  /**
   * Validate State Transition
   *
   * @param {
   * DataContractCreateTransition
   * |RawDataContractCreateTransition
   * |DocumentsBatchTransition
   * |RawDocumentsBatchTransition
   * } stateTransition
   * @return {ValidationResult}
   */
  async validate(stateTransition) {
    const result = await this.validateStructure(stateTransition);

    if (!result.isValid()) {
      return result;
    }

    let stateTransitionModel = stateTransition;

    if (!(stateTransition instanceof AbstractStateTransition)) {
      stateTransitionModel = this.createStateTransition(stateTransition, {
        fromJSON: true,
      });
    }

    return this.validateData(stateTransitionModel);
  }

  /**
   * Validate State Transition Structure
   *
   * @param {
   *  DataContractCreateTransition
   * |RawDataContractCreateTransition
   * |RawDocumentsBatchTransition
   * |DocumentsBatchTransition
   * } stateTransition
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

    return this.validateStateTransitionStructure(stateTransition);
  }

  /**
   * Validate State Transition Data
   *
   * @param {
   *  DataContractCreateTransition|DocumentsBatchTransition|IdentityCreateTransition
   *  |RawDataContractCreateTransition|RawDocumentsBatchTransition|RawIdentityCreateTransition
   *  |RawIdentityTopUpTransition
   *  } stateTransition
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
    let stateTransitionModel = stateTransition;

    if (!(stateTransition instanceof AbstractStateTransition)) {
      stateTransitionModel = await this.createFromObject(stateTransition);
    }

    return this.validateStateTransitionData(stateTransitionModel);
  }

  /**
   * Validate State Transition Fee
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

    let stateTransitionModel = stateTransition;

    if (!(stateTransition instanceof AbstractStateTransition)) {
      stateTransitionModel = await this.createFromObject(stateTransition);
    }

    return this.validateStateTransitionFee(stateTransitionModel);
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

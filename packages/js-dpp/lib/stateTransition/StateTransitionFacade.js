const $RefParser = require('@apidevtools/json-schema-ref-parser');
const jsonPatch = require('fast-json-patch');
const jsonSchemaDiffValidator = require('json-schema-diff-validator');
const { Signer: { verifyHashSignature } } = require('@dashevo/dashcore-lib');

const MissingOptionError = require('../errors/MissingOptionError');

const StateTransitionFactory = require('./StateTransitionFactory');

const AbstractStateTransition = require('./AbstractStateTransition');

const stateTransitionTypes = require('./stateTransitionTypes');

const createStateTransitionFactory = require('./createStateTransitionFactory');

const validateDataContractFactory = require('../dataContract/validation/validateDataContractFactory');
const validateDataContractPatternsFactory = require('../dataContract/validation/validateDataContractPatternsFactory');
const validateDataContractCreateTransitionBasicFactory = require('../dataContract/stateTransition/DataContractCreateTransition/validation/basic/validateDataContractCreateTransitionBasicFactory');
const validateStateTransitionBasicFactory = require('./validation/validateStateTransitionBasicFactory');
const validateDataContractCreateTransitionStateFactory = require('../dataContract/stateTransition/DataContractCreateTransition/validation/state/validateDataContractCreateTransitionStateFactory');
const validateStateTransitionStateFactory = require('./validation/validateStateTransitionStateFactory');
const validateDocumentsBatchTransitionBasicFactory = require('../document/stateTransition/DocumentsBatchTransition/validation/basic/validateDocumentsBatchTransitionBasicFactory');
const validateIdentityCreateTransitionStateFactory = require('../identity/stateTransition/IdentityCreateTransition/validation/state/validateIdentityCreateTransitionStateFactory');
const validateIdentityTopUpTransitionStateFactory = require('../identity/stateTransition/IdentityTopUpTransition/validation/state/validateIdentityTopUpTransitionStateFactory');
const validateIdentityUpdateTransitionStateFactory = require('../identity/stateTransition/IdentityUpdateTransition/validation/state/validateIdentityUpdateTransitionStateFactory');
const validateIdentityCreateTransitionBasicFactory = require('../identity/stateTransition/IdentityCreateTransition/validation/basic/validateIdentityCreateTransitionBasicFactory');
const validateIdentityTopUpTransitionBasicFactory = require('../identity/stateTransition/IdentityTopUpTransition/validation/basic/validateIdentityTopUpTransitionBasicFactory');
const validateIdentityUpdateTransitionBasicFactory = require('../identity/stateTransition/IdentityUpdateTransition/validation/basic/validateIdentityUpdateTransitionBasicFactory');
const validateStateTransitionIdentitySignatureFactory = require('./validation/validateStateTransitionIdentitySignatureFactory');
const validateStateTransitionFeeFactory = require('./validation/validateStateTransitionFeeFactory');

const enrichDataContractWithBaseSchema = require('../dataContract/enrichDataContractWithBaseSchema');
const findDuplicatesById = require('../document/stateTransition/DocumentsBatchTransition/validation/basic/findDuplicatesById');
const findDuplicatesByIndices = require('../document/stateTransition/DocumentsBatchTransition/validation/basic/findDuplicatesByIndices');

const validateDocumentsBatchTransitionStateFactory = require('../document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsBatchTransitionStateFactory');
const fetchDocumentsFactory = require('../document/stateTransition/DocumentsBatchTransition/validation/state/fetchDocumentsFactory');
const validateDocumentsUniquenessByIndicesFactory = require('../document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsUniquenessByIndicesFactory');
const validatePartialCompoundIndices = require('../document/stateTransition/DocumentsBatchTransition/validation/basic/validatePartialCompoundIndices');
const getDataTriggersFactory = require('../dataTrigger/getDataTriggersFactory');
const executeDataTriggersFactory = require('../document/stateTransition/DocumentsBatchTransition/validation/state/executeDataTriggersFactory');
const validatePublicKeysFactory = require('../identity/validation/validatePublicKeysFactory');
const validatePublicKeysState = require('../identity/stateTransition/IdentityUpdateTransition/validation/state/validatePublicKeysState');
const validateRequiredPurposeAndSecurityLevelFactory = require('../identity/validation/validateRequiredPurposeAndSecurityLevelFactory');
const validateDataContractMaxDepthFactory = require('../dataContract/validation/validateDataContractMaxDepthFactory');

const applyStateTransitionFactory = require('./applyStateTransitionFactory');

const applyDataContractCreateTransitionFactory = require(
  '../dataContract/stateTransition/DataContractCreateTransition/applyDataContractCreateTransitionFactory',
);
const applyDataContractUpdateTransitionFactory = require(
  '../dataContract/stateTransition/DataContractUpdateTransition/applyDataContractUpdateTransitionFactory',
);

const applyDocumentsBatchTransitionFactory = require(
  '../document/stateTransition/DocumentsBatchTransition/applyDocumentsBatchTransitionFactory',
);

const applyIdentityCreateTransitionFactory = require(
  '../identity/stateTransition/IdentityCreateTransition/applyIdentityCreateTransitionFactory',
);

const applyIdentityTopUpTransitionFactory = require(
  '../identity/stateTransition/IdentityTopUpTransition/applyIdentityTopUpTransitionFactory',
);

const applyIdentityUpdateTransitionFactory = require(
  '../identity/stateTransition/IdentityUpdateTransition/applyIdentityUpdateTransitionFactory',
);
const validateInstantAssetLockProofStructureFactory = require('../identity/stateTransition/assetLockProof/instant/validateInstantAssetLockProofStructureFactory');
const calculateStateTransitionFee = require('./fee/calculateStateTransitionFee');
const InstantAssetLockProof = require('../identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const ChainAssetLockProof = require('../identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const validateChainAssetLockProofStructureFactory = require('../identity/stateTransition/assetLockProof/chain/validateChainAssetLockProofStructureFactory');
const fetchAssetLockTransactionOutputFactory = require('../identity/stateTransition/assetLockProof/fetchAssetLockTransactionOutputFactory');
const validateAssetLockTransactionFactory = require('../identity/stateTransition/assetLockProof/validateAssetLockTransactionFactory');

const ValidationResult = require('../validation/ValidationResult');
const AbstractStateTransitionIdentitySigned = require('./AbstractStateTransitionIdentitySigned');
const validateStateTransitionKeySignatureFactory = require('./validation/validateStateTransitionKeySignatureFactory');

const fetchAssetLockPublicKeyHashFactory = require('../identity/stateTransition/assetLockProof/fetchAssetLockPublicKeyHashFactory');

const decodeProtocolEntityFactory = require('../decodeProtocolEntityFactory');
const protocolVersion = require('../version/protocolVersion');
const validateProtocolVersionFactory = require('../version/validateProtocolVersionFactory');
const validateDataContractUpdateTransitionBasicFactory = require('../dataContract/stateTransition/DataContractUpdateTransition/validation/basic/validateDataContractUpdateTransitionBasicFactory');
const validateDataContractUpdateTransitionStateFactory = require('../dataContract/stateTransition/DataContractUpdateTransition/validation/state/validateDataContractUpdateTransitionStateFactory');
const validateIndicesAreBackwardCompatible = require('../dataContract/stateTransition/DataContractUpdateTransition/validation/basic/validateIndicesAreBackwardCompatible');
const getPropertyDefinitionByPath = require('../dataContract/getPropertyDefinitionByPath');

const identityJsonSchema = require('../../schema/identity/stateTransition/publicKey.json');
const validatePublicKeySignaturesFactory = require('../identity/stateTransition/validatePublicKeySignaturesFactory');
const StateTransitionExecutionContext = require('./StateTransitionExecutionContext');

class StateTransitionFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {RE2} RE2
   * @param {BlsSignatures} bls
   */
  constructor(dpp, RE2, bls) {
    this.stateRepository = dpp.getStateRepository();

    const validator = dpp.getJsonSchemaValidator();

    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);
    const validateDataContractPatterns = validateDataContractPatternsFactory(RE2);

    const validateProtocolVersion = validateProtocolVersionFactory(
      dpp,
      protocolVersion.compatibility,
    );
    const validateDataContract = validateDataContractFactory(
      validator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
      validateDataContractPatterns,
      validateProtocolVersion,
      getPropertyDefinitionByPath,
    );

    this.validateStateTransitionIdentitySignature = validateStateTransitionIdentitySignatureFactory(
      this.stateRepository,
    );

    const fetchAssetLockTransactionOutput = fetchAssetLockTransactionOutputFactory(
      this.stateRepository,
    );

    const fetchAssetLockPublicKeyHash = fetchAssetLockPublicKeyHashFactory(
      fetchAssetLockTransactionOutput,
    );

    this.validateStateTransitionKeySignature = validateStateTransitionKeySignatureFactory(
      verifyHashSignature,
      fetchAssetLockPublicKeyHash,
    );

    // eslint-disable-next-line max-len
    const validateDataContractCreateTransitionBasic = validateDataContractCreateTransitionBasicFactory(
      validator,
      validateDataContract,
      validateProtocolVersion,
    );

    // eslint-disable-next-line max-len
    const validateDataContractUpdateTransitionBasic = validateDataContractUpdateTransitionBasicFactory(
      validator,
      validateDataContract,
      validateProtocolVersion,
      this.stateRepository,
      jsonSchemaDiffValidator,
      validateIndicesAreBackwardCompatible,
      jsonPatch,
    );

    this.createStateTransition = createStateTransitionFactory(this.stateRepository);

    const validateDocumentsBatchTransitionBasic = (
      validateDocumentsBatchTransitionBasicFactory(
        findDuplicatesById,
        findDuplicatesByIndices,
        this.stateRepository,
        validator,
        enrichDataContractWithBaseSchema,
        validatePartialCompoundIndices,
        validateProtocolVersion,
      )
    );

    const validateAssetLockTransaction = validateAssetLockTransactionFactory(this.stateRepository);

    const validateInstantAssetLockProofStructure = validateInstantAssetLockProofStructureFactory(
      validator,
      this.stateRepository,
      validateAssetLockTransaction,
    );

    const validateChainAssetLockProofStructure = validateChainAssetLockProofStructureFactory(
      validator,
      this.stateRepository,
      validateAssetLockTransaction,
    );

    const proofValidationFunctionsByType = {
      [InstantAssetLockProof.type]: validateInstantAssetLockProofStructure,
      [ChainAssetLockProof.type]: validateChainAssetLockProofStructure,
    };

    const validatePublicKeys = validatePublicKeysFactory(
      validator,
      identityJsonSchema,
      bls,
    );

    const validateRequiredPurposeAndSecurityLevel = (
      validateRequiredPurposeAndSecurityLevelFactory()
    );

    const validatePublicKeySignatures = validatePublicKeySignaturesFactory(
      this.createStateTransition,
    );

    const validateIdentityCreateTransitionBasic = (
      validateIdentityCreateTransitionBasicFactory(
        validator,
        validatePublicKeys,
        validateRequiredPurposeAndSecurityLevel,
        proofValidationFunctionsByType,
        validateProtocolVersion,
        validatePublicKeySignatures,
      )
    );

    const validateIdentityTopUpTransitionBasic = (
      validateIdentityTopUpTransitionBasicFactory(
        validator,
        proofValidationFunctionsByType,
        validateProtocolVersion,
      )
    );

    const validateIdentityUpdateTransitionBasic = validateIdentityUpdateTransitionBasicFactory(
      validator,
      validateProtocolVersion,
      validatePublicKeys,
      validatePublicKeySignatures,
    );

    const validationFunctionsByType = {
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractCreateTransitionBasic,
      [stateTransitionTypes.DATA_CONTRACT_UPDATE]: validateDataContractUpdateTransitionBasic,
      [stateTransitionTypes.DOCUMENTS_BATCH]: validateDocumentsBatchTransitionBasic,
      [stateTransitionTypes.IDENTITY_CREATE]: validateIdentityCreateTransitionBasic,
      [stateTransitionTypes.IDENTITY_TOP_UP]: validateIdentityTopUpTransitionBasic,
      [stateTransitionTypes.IDENTITY_UPDATE]: validateIdentityUpdateTransitionBasic,
    };

    this.validateStateTransitionBasic = validateStateTransitionBasicFactory(
      validationFunctionsByType,
      this.createStateTransition,
    );

    const validateDataContractCreateTransitionState = (
      validateDataContractCreateTransitionStateFactory(
        this.stateRepository,
      )
    );

    const validateDataContractUpdateTransitionState = (
      validateDataContractUpdateTransitionStateFactory(
        this.stateRepository,
      )
    );

    const validateIdentityCreateTransitionState = validateIdentityCreateTransitionStateFactory(
      this.stateRepository,
    );

    const validateIdentityTopUpTransitionState = validateIdentityTopUpTransitionStateFactory();

    const validateIdentityUpdateTransitionState = validateIdentityUpdateTransitionStateFactory(
      this.stateRepository,
      validatePublicKeysState,
      validateRequiredPurposeAndSecurityLevel,
    );

    const fetchDocuments = fetchDocumentsFactory(
      this.stateRepository,
    );

    const validateDocumentsUniquenessByIndices = validateDocumentsUniquenessByIndicesFactory(
      this.stateRepository,
    );

    const getDataTriggers = getDataTriggersFactory();

    const executeDataTriggers = executeDataTriggersFactory(
      getDataTriggers,
    );

    const validateDocumentsBatchTransitionState = validateDocumentsBatchTransitionStateFactory(
      this.stateRepository,
      fetchDocuments,
      validateDocumentsUniquenessByIndices,
      executeDataTriggers,
    );

    this.validateStateTransitionState = validateStateTransitionStateFactory({
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractCreateTransitionState,
      [stateTransitionTypes.DATA_CONTRACT_UPDATE]: validateDataContractUpdateTransitionState,
      [stateTransitionTypes.DOCUMENTS_BATCH]: validateDocumentsBatchTransitionState,
      [stateTransitionTypes.IDENTITY_CREATE]: validateIdentityCreateTransitionState,
      [stateTransitionTypes.IDENTITY_TOP_UP]: validateIdentityTopUpTransitionState,
      [stateTransitionTypes.IDENTITY_UPDATE]: validateIdentityUpdateTransitionState,
    });

    this.validateStateTransitionFee = validateStateTransitionFeeFactory(
      this.stateRepository,
      calculateStateTransitionFee,
      fetchAssetLockTransactionOutput,
    );

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    this.factory = new StateTransitionFactory(
      this.validateStateTransitionBasic,
      this.createStateTransition,
      dpp,
      decodeProtocolEntity,
    );

    const applyDataContractCreateTransition = applyDataContractCreateTransitionFactory(
      this.stateRepository,
    );

    const applyDataContractUpdateTransition = applyDataContractUpdateTransitionFactory(
      this.stateRepository,
    );

    const applyDocumentsBatchTransition = applyDocumentsBatchTransitionFactory(
      this.stateRepository,
      fetchDocuments,
    );

    const applyIdentityCreateTransition = applyIdentityCreateTransitionFactory(
      this.stateRepository,
      fetchAssetLockTransactionOutput,
    );

    const applyIdentityTopUpTransition = applyIdentityTopUpTransitionFactory(
      this.stateRepository,
      fetchAssetLockTransactionOutput,
    );

    const applyIdentityUpdateTransition = applyIdentityUpdateTransitionFactory(
      this.stateRepository,
    );

    this.applyStateTransition = applyStateTransitionFactory(
      applyDataContractCreateTransition,
      applyDataContractUpdateTransition,
      applyDocumentsBatchTransition,
      applyIdentityCreateTransition,
      applyIdentityTopUpTransition,
      applyIdentityUpdateTransition,
    );
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   * @param {Object} [options]
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
   * @param {Object} [options]
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
   * @param {Object} [options]
   * @param {boolean} [options.basic=true]
   * @param {boolean} [options.signature=true]
   * @param {boolean} [options.fee=true]
   * @param {boolean} [options.state=true]
   * @return {Promise<ValidationResult>}
   */
  async validate(stateTransition, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = {
      basic: true,
      signature: true,
      fee: true,
      state: true,
      ...options,
    };

    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    // Convert raw state transition to the model
    let stateTransitionModel = stateTransition;

    if (!(stateTransition instanceof AbstractStateTransition)) {
      stateTransitionModel = await this.createStateTransition(stateTransition);
    }

    const result = new ValidationResult();

    // Basic validation
    if (options.basic) {
      result.merge(
        await this.validateBasic(stateTransition),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    // Signature validation
    if (options.signature) {
      result.merge(
        await this.validateSignature(stateTransitionModel),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    // Fee validation
    if (options.fee) {
      result.merge(
        await this.validateFee(stateTransitionModel),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    // Validate against existing state
    if (options.state) {
      result.merge(
        await this.validateState(stateTransitionModel),
      );
    }

    return result;
  }

  /**
   * Validate State Transition structure and data
   *
   * @param {AbstractStateTransition|RawStateTransition} stateTransition
   * @return {Promise<ValidationResult>}
   */
  async validateBasic(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    let rawStateTransition;
    let executionContext;

    if (stateTransition instanceof AbstractStateTransition) {
      rawStateTransition = stateTransition.toObject();
      executionContext = stateTransition.getExecutionContext();
    } else {
      rawStateTransition = stateTransition;
    }

    return this.validateStateTransitionBasic(
      rawStateTransition,
      executionContext || new StateTransitionExecutionContext(),
    );
  }

  /**
   * Validate State Transition signature and ownership
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @return {Promise<ValidationResult>}
   */
  async validateSignature(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    if (stateTransition instanceof AbstractStateTransitionIdentitySigned) {
      return this.validateStateTransitionIdentitySignature(stateTransition);
    }

    return this.validateStateTransitionKeySignature(stateTransition);
  }

  /**
   * Validate State Transition fee
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @return {Promise<ValidationResult>}
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
   * Validate State Transition against existing state
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @return {Promise<ValidationResult>}
   */
  async validateState(stateTransition) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate State Transition because State Repository is not set, use'
        + ' setStateRepository method',
      );
    }

    return this.validateStateTransitionState(stateTransition);
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

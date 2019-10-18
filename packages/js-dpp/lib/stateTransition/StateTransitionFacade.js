const MissingOptionError = require('../errors/MissingOptionError');

const StateTransitionFactory = require('./StateTransitionFactory');

const AbstractStateTransition = require('./AbstractStateTransition');

const stateTransitionTypes = require('./stateTransitionTypes');

const dataContractStateTransitionSchema = require('../../schema/stateTransition/data-contract');
const documentsStateTransitionSchema = require('../../schema/stateTransition/documents');

const createDataContract = require('../dataContract/createDataContract');
const createStateTransitionFactory = require('./createStateTransitionFactory');
const validateDataContractFactory = require('../dataContract/validateDataContractFactory');
const validateDataContractSTStructureFactory = require('../dataContract/stateTransition/validation/validateDataContractSTStructureFactory');
const validateStateTransitionStructureFactory = require('./validation/validateStateTransitionStructureFactory');
const validateDataContractSTDataFactory = require('../dataContract/stateTransition/validation/validateDataContractSTDataFactory');
const validateStateTransitionDataFactory = require('./validation/validateStateTransitionDataFactory');
const validateDocumentsSTStructureFactory = require('../document/stateTransition/validation/structure/validateDocumentsSTStructureFactory');
const validateDocumentFactory = require('../document/validateDocumentFactory');

const enrichDataContractWithBaseDocument = require('../document/enrichDataContractWithBaseDocument');
const findDuplicateDocumentsById = require('../document/stateTransition/validation/structure/findDuplicateDocumentsById');
const findDuplicateDocumentsByIndices = require('../document/stateTransition/validation/structure/findDuplicateDocumentsByIndices');

const validateBlockchainUserFactory = require('./validation/validateBlockchainUserFactory');

class StateTransitionFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {JsonSchemaValidator} validator
   */
  constructor(dpp, validator) {
    this.dpp = dpp;
    this.validator = validator;

    const validateDataContract = validateDataContractFactory(
      this.validator,
    );

    this.validateDataContractSTStructure = validateDataContractSTStructureFactory(
      validateDataContract,
    );

    this.validateDocument = validateDocumentFactory(
      this.validator,
      enrichDataContractWithBaseDocument,
    );

    this.createStateTransition = createStateTransitionFactory(
      createDataContract,
    );
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawDataContractStateTransition|RawDocumentsStateTransition} rawStateTransition
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  async createFromObject(rawStateTransition, options = {}) {
    return this.getFactory().createFromObject(rawStateTransition, options);
  }

  /**
   * Create State Transition from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  async createFromSerialized(payload, options = {}) {
    return this.getFactory().createFromSerialized(payload, options);
  }

  /**
   * Validate State Transition
   *
   * @param {
   * DataContractStateTransition
   * |RawDataContractStateTransition
   * |DocumentsStateTransition
   * |RawDocumentsStateTransition
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
      stateTransitionModel = this.createStateTransition(stateTransition);
    }

    return this.validateData(stateTransitionModel);
  }

  /**
   * Validate State Transition Structure
   *
   * @param {
   *  DataContractStateTransition
   * |RawDataContractStateTransition
   * |RawDocumentsStateTransition
   * |DocumentsStateTransition
   * } stateTransition
   * @return {ValidationResult}
   */
  async validateStructure(stateTransition) {
    const validateStateTransitionStructure = this.createValidateStateTransitionStructure();

    return validateStateTransitionStructure(stateTransition);
  }

  /**
   * Validate State Transition Data
   *
   * @param {DataContractStateTransition|DocumentsStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async validateData(stateTransition) {
    const validateStateTransitionData = this.createValidateStateTransitionData();

    return validateStateTransitionData(stateTransition);
  }

  /**
   * @private
   * @return {validateStateTransitionStructure}
   */
  createValidateStateTransitionStructure() {
    const dataProvider = this.dpp.getDataProvider();

    if (!dataProvider) {
      throw new MissingOptionError(
        'dataProvider',
        'Can\'t validate State Transition data because Data Provider is not set, use'
        + ' setDataProvider method',
      );
    }

    const validateDocumentsSTStructure = validateDocumentsSTStructureFactory(
      this.validateDocument,
      findDuplicateDocumentsById,
      findDuplicateDocumentsByIndices,
      this.dpp.getDataProvider(),
    );

    const typeExtensions = {
      [stateTransitionTypes.DATA_CONTRACT]: {
        function: this.validateDataContractSTStructure,
        schema: dataContractStateTransitionSchema,
      },
      [stateTransitionTypes.DOCUMENTS]: {
        function: validateDocumentsSTStructure,
        schema: documentsStateTransitionSchema,
      },
    };

    return validateStateTransitionStructureFactory(
      this.validator,
      typeExtensions,
    );
  }

  /**
   * @private
   * @return {validateStateTransitionData}
   */
  createValidateStateTransitionData() {
    const dataProvider = this.dpp.getDataProvider();

    if (!dataProvider) {
      throw new MissingOptionError(
        'dataProvider',
        'Can\'t validate State Transition data because Data Provider is not set, use'
        + ' setDataProvider method',
      );
    }

    const validateBlockchainUser = validateBlockchainUserFactory(
      dataProvider,
    );

    const validateDataContractSTData = validateDataContractSTDataFactory(
      dataProvider,
      validateBlockchainUser,
    );

    return validateStateTransitionDataFactory(
      validateDataContractSTData,
    );
  }

  /**
   * @private
   * @return {StateTransitionFactory}
   */
  getFactory() {
    return new StateTransitionFactory(
      this.createValidateStateTransitionStructure(),
      this.createStateTransition,
    );
  }
}

module.exports = StateTransitionFacade;

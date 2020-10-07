const $RefParser = require('@apidevtools/json-schema-ref-parser');

const DataContract = require('./DataContract');
const DataContractFactory = require('./DataContractFactory');
const validateDataContractFactory = require('./validateDataContractFactory');
const enrichDataContractWithBaseSchema = require('./enrichDataContractWithBaseSchema');
const validateDataContractMaxDepthFactory = require('./stateTransition/validation/validateDataContractMaxDepthFactory');

class DataContractFacade {
  /**
   * @param {JsonSchemaValidator} jsonSchemaValidator
   */
  constructor(jsonSchemaValidator) {
    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    this.validateDataContract = validateDataContractFactory(
      jsonSchemaValidator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
    );

    this.factory = new DataContractFactory(
      this.validateDataContract,
    );
  }

  /**
   * Create Data Contract
   *
   * @param {Buffer} ownerId
   * @param {Object} documents
   * @return {DataContract}
   */
  create(ownerId, documents) {
    return this.factory.create(ownerId, documents);
  }

  /**
   * Create Data Contract from plain object
   *
   * @param {RawDataContract} rawDataContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromObject(rawDataContract, options = { }) {
    return this.factory.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract from buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromBuffer(buffer, options = { }) {
    return this.factory.createFromBuffer(buffer, options);
  }

  /**
   * Create Data Contract State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractCreateTransition}
   */
  createStateTransition(dataContract) {
    return this.factory.createStateTransition(dataContract);
  }

  /**
   * Validate Data Contract
   *
   * @param {DataContract|RawDataContract} dataContract
   * @return {Promise<ValidationResult>}
   */
  async validate(dataContract) {
    let rawDataContract;
    if (dataContract instanceof DataContract) {
      rawDataContract = dataContract.toObject();
    } else {
      rawDataContract = dataContract;
    }

    return this.validateDataContract(rawDataContract);
  }
}

module.exports = DataContractFacade;

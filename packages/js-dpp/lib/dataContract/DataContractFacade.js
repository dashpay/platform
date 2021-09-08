const $RefParser = require('@apidevtools/json-schema-ref-parser');

const DataContract = require('./DataContract');
const DataContractFactory = require('./DataContractFactory');
const validateDataContractFactory = require('./validation/validateDataContractFactory');
const enrichDataContractWithBaseSchema = require('./enrichDataContractWithBaseSchema');
const validateDataContractMaxDepthFactory = require('./validation/validateDataContractMaxDepthFactory');
const validateDataContractPatternsFactory = require('./validation/validateDataContractPatternsFactory');
const decodeProtocolEntityFactory = require('../decodeProtocolEntityFactory');

const protocolVersion = require('../version/protocolVersion');
const validateProtocolVersionFactory = require('../version/validateProtocolVersionFactory');

class DataContractFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {RE2} RE2
   */
  constructor(dpp, RE2) {
    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    const validateDataContractPatterns = validateDataContractPatternsFactory(RE2);

    const validateProtocolVersion = validateProtocolVersionFactory(
      dpp,
      protocolVersion.compatibility,
    );

    this.validateDataContract = validateDataContractFactory(
      dpp.getJsonSchemaValidator(),
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
      validateDataContractPatterns,
      RE2,
      validateProtocolVersion,
    );

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    this.factory = new DataContractFactory(
      dpp,
      this.validateDataContract,
      decodeProtocolEntity,
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

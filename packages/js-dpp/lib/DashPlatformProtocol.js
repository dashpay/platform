const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DataContractFacade = require('./dataContract/DataContractFacade');
const DocumentFacade = require('./document/DocumentFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

const userIdPropertyName = Symbol('userId');
const dataContractPropertyName = Symbol('dataContract');
const dataProviderPropertyName = Symbol('dataProvider');

/**
 * @class DashPlatformProtocol
 */
class DashPlatformProtocol {
  /**
   * @param {string} [options.userId]
   * @param {DataContract} [options.dataContract]
   * @param {DataProvider} [options.dataProvider]
   */
  constructor(options = {}) {
    this[userIdPropertyName] = options.userId;
    this[dataContractPropertyName] = options.dataContract;
    this[dataProviderPropertyName] = options.dataProvider;

    const validator = new JsonSchemaValidator(new Ajv());

    this.initializeFacades(validator);
  }

  /**
   * @private
   * @param {JsonSchemaValidator} validator
   */
  initializeFacades(validator) {
    this.dataContract = new DataContractFacade(validator);

    this.document = new DocumentFacade(this, validator);

    this.packet = new STPacketFacade(this, validator);

    this.packetHeader = new STPacketHeaderFacade(validator);
  }

  /**
   * Set User ID
   *
   * @param {string} userId
   * @return {DashPlatformProtocol}
   */
  setUserId(userId) {
    this[userIdPropertyName] = userId;

    return this;
  }

  /**
   * Get User ID
   *
   * @return {string}
   */
  getUserId() {
    return this[userIdPropertyName];
  }

  /**
   * Set Data Contract
   *
   * @param {DataContract} dataContract
   * @return {DashPlatformProtocol}
   */
  setDataContract(dataContract) {
    this[dataContractPropertyName] = dataContract;

    return this;
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this[dataContractPropertyName];
  }

  /**
   * Set Data Provider
   *
   * @param {DataProvider} dataProvider
   * @return {DashPlatformProtocol}
   */
  setDataProvider(dataProvider) {
    this[dataProviderPropertyName] = dataProvider;

    return this;
  }

  /**
   * Get Data Provider
   *
   * @return {DataProvider}
   */
  getDataProvider() {
    return this[dataProviderPropertyName];
  }
}

module.exports = DashPlatformProtocol;

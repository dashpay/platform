const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const ContractFacade = require('./contract/ContractFacade');
const DocumentFacade = require('./document/DocumentFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

const userIdPropertyName = Symbol('userId');
const contractPropertyName = Symbol('contract');
const dataProviderPropertyName = Symbol('dataProvider');

/**
 * @class DashPlatformProtocol
 */
class DashPlatformProtocol {
  /**
   * @param {string} [options.userId]
   * @param {Contract} [options.contract]
   * @param {DataProvider} [options.dataProvider]
   */
  constructor(options = {}) {
    this[userIdPropertyName] = options.userId;
    this[contractPropertyName] = options.contract;
    this[dataProviderPropertyName] = options.dataProvider;

    const validator = new JsonSchemaValidator(new Ajv());

    this.initializeFacades(validator);
  }

  /**
   * @private
   * @param {JsonSchemaValidator} validator
   */
  initializeFacades(validator) {
    this.contract = new ContractFacade(validator);

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
   * Set Contract
   *
   * @param {Contract} contract
   * @return {DashPlatformProtocol}
   */
  setContract(contract) {
    this[contractPropertyName] = contract;

    return this;
  }

  /**
   * Get Contract
   *
   * @return {Contract}
   */
  getContract() {
    return this[contractPropertyName];
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

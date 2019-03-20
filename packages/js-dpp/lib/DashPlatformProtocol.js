const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DPContractFacade = require('./contract/DPContractFacade');
const DocumentFacade = require('./document/DocumentFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

/**
 * @class DashPlatformProtocol
 */
class DashPlatformProtocol {
  /**
   * @param {string} [options.userId]
   * @param {DPContract} [options.dpContract]
   * @param {DataProvider} [options.dataProvider]
   */
  constructor(options = {}) {
    this.userId = options.userId;
    this.dpContract = options.dpContract;
    this.dataProvider = options.dataProvider;

    const validator = new JsonSchemaValidator(new Ajv());

    this.initializeFacades(validator);
  }

  /**
   * @private
   * @param {JsonSchemaValidator} validator
   */
  initializeFacades(validator) {
    this.contract = new DPContractFacade(validator);

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
    this.userId = userId;

    return this;
  }

  /**
   * Get User ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Set DP Contract
   *
   * @param {DPContract} dpContract
   * @return {DashPlatformProtocol}
   */
  setDPContract(dpContract) {
    this.dpContract = dpContract;

    return this;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
  }

  /**
   * Set Data Provider
   *
   * @param {DataProvider} dataProvider
   * @return {DashPlatformProtocol}
   */
  setDataProvider(dataProvider) {
    this.dataProvider = dataProvider;

    return this;
  }

  /**
   * Get Data Provider
   *
   * @return {DataProvider}
   */
  getDataProvider() {
    return this.dataProvider;
  }
}

module.exports = DashPlatformProtocol;

const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DapContractFacade = require('./dapContract/DapContractFacade');
const DapObjectFacade = require('./dapObject/DapObjectFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

const FACADES = ['contract', 'object', 'packet', 'packetHeader'];

class DashApplicationProtocol {
  /**
   * @param {string} [options.userId]
   * @param {string} [options.dapContract]
   * @param {string} [options.dapContractId]
   * @param {AbstractDataProvider} [options.dataProvider]
   */
  constructor(options) {
    this.userId = options.userId;
    this.dapContractId = options.dapContractId;
    this.dapContract = options.dapContract;
    this.dataProvider = options.dataProvider;

    const validator = new JsonSchemaValidator(new Ajv());

    this.initializeFacades(validator);
  }

  /**
   * @private
   * @param {JsonSchemaValidator} validator
   */
  initializeFacades(validator) {
    this.contract = new DapContractFacade(validator);

    this.object = new DapObjectFacade(this, validator);

    this.packet = new STPacketFacade(this, validator);

    this.packetHeader = new STPacketHeaderFacade(validator);
  }

  /**
   * @private
   */
  updateFacades() {
    FACADES.forEach((name) => {
      this[name].updateDependencies(this);
    });
  }

  /**
   * Set User ID
   *
   * @param {string} userId
   * @return {DashApplicationProtocol}
   */
  setUserId(userId) {
    this.userId = userId;

    this.updateFacades();

    return this;
  }

  /**
   * Get User ID
   *
   * @return {string|undefined}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Set Dap Contract ID
   *
   * @return {DashApplicationProtocol}
   */
  setDapContractId(dapContractId) {
    this.dapContractId = dapContractId;
    this.dapContract = null;

    this.updateFacades();

    return this;
  }

  /**
   * Get Dap Contract ID
   *
   * @return {string|undefined}
   */
  getDapContractId() {
    return this.dapContractId;
  }

  /**
   * Set Dap Contract
   *
   * @param {DapContract} dapContract
   * @return {DashApplicationProtocol}
   */
  setDapContract(dapContract) {
    this.dapContract = dapContract;

    this.updateFacades();

    return this;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContract() {
    if (!this.dapContract) {
      if (!this.dapContractId) {
        throw new Error('dapContractId is not set');
      }

      this.dapContract = this.getDataProvider().fetchDapContract(this.dapContractId);
    }

    return this.dapContract;
  }

  /**
   * Set Data Provider
   *
   * @param {AbstractDataProvider} dataProvider
   * @return {DashApplicationProtocol}
   */
  setDataProvider(dataProvider) {
    this.dataProvider = dataProvider;

    this.updateFacades();

    return this;
  }

  /**
   * Get Data Provider
   *
   * @return AbstractDataProvider
   */
  getDataProvider() {
    return this.dataProvider;
  }
}

module.exports = DashApplicationProtocol;

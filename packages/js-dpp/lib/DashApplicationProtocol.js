const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DapContractFacade = require('./dapContract/DapContractFacade');
const DapObjectFacade = require('./dapObject/DapObjectFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

class DashApplicationProtocol {
  /**
   * @param {string} [options.userId]
   * @param {string} [options.dapContract]
   * @param {string} [options.dapContractId]
   * @param {AbstractDataProvider} [options.dataProvider]
   */
  constructor(options = {}) {
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
    DashApplicationProtocol.FACADES.forEach((name) => {
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
  async getDapContract() {
    if (!this.dapContract && this.dapContractId) {
      this.dapContract = await this.getDataProvider().fetchDapContract(this.dapContractId);

      if (!this.dapContract) {
        throw new Error();
      }
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

DashApplicationProtocol.FACADES = ['contract', 'object', 'packet', 'packetHeader'];

module.exports = DashApplicationProtocol;

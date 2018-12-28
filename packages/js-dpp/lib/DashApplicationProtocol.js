const Ajv = require('ajv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DapContractFacade = require('./dapContract/DapContractFacade');
const DapObjectFacade = require('./dapObject/DapObjectFacade');
const STPacketFacade = require('./stPacket/STPacketFacade');
const STPacketHeaderFacade = require('./stPacketHeader/STPacketHeaderFacade');

/**
 * @classdesc DataProvider interface definition
 *
 * @name DataProvider
 * @class
 */

/**
 * Fetch Dap Contract by ID
 *
 * @method
 * @name DataProvider#fetchDapContract
 * @param {string} id
 * @returns {DapContract|null}
 */

/**
 * Fetch DAP Objects by contract ID and type
 *
 * @method
 * @name DataProvider#fetchDapObjects
 * @param {string} dapContractId
 * @param {string} type
 * @param {{ where: Object }} [options]
 * @returns {DapObject[]}
 */

/**
 * Fetch transaction by ID
 *
 * @method
 * @name DataProvider#fetchTransaction
 * @param {string} id
 * @returns {{ confirmations: number }}
 */


class DashApplicationProtocol {
  /**
   * @param {string} [options.userId]
   * @param {DapContract} [options.dapContract]
   * @param {DataProvider} [options.dataProvider]
   */
  constructor(options = {}) {
    this.userId = options.userId;
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
   * Set User ID
   *
   * @param {string} userId
   * @return {DashApplicationProtocol}
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
   * Set Dap Contract
   *
   * @param {DapContract} dapContract
   * @return {DashApplicationProtocol}
   */
  setDapContract(dapContract) {
    this.dapContract = dapContract;

    return this;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContract() {
    return this.dapContract;
  }

  /**
   * Set Data Provider
   *
   * @param {DataProvider} dataProvider
   * @return {DashApplicationProtocol}
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

module.exports = DashApplicationProtocol;

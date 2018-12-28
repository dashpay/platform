const validateSTPacketFactory = require('./validation/validateSTPacketFactory');

const validateSTPacketDapContractsFactory = require('./validation/validateSTPacketDapContractsFactory');
const validateSTPacketDapObjectsFactory = require('./validation/validateSTPacketDapObjectsFactory');

const findDuplicatedDapObjects = require('./validation/findDuplicatedDapObjects');
const createDapContract = require('../dapContract/createDapContract');

const verifySTPacketFactory = require('./verification/verifySTPacketFactory');
const verifyDapContractFactory = require('./verification/verifyDapContractFactory');
const verifyDapObjectsFactory = require('./verification/verifyDapObjectsFactory');
const fetchDapObjectsByObjectsFactory = require('./verification/fetchDapObjectsByObjectsFactory');

const STPacketFactory = require('./STPacketFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class STPacketFacade {
  /**
   * @param {DashApplicationProtocol} dap
   * @param {JsonSchemaValidator} validator
   */
  constructor(dap, validator) {
    this.dap = dap;

    const validateSTPacketDapContracts = validateSTPacketDapContractsFactory(
      dap.contract.validateDapContract,
      createDapContract,
    );

    const validateSTPacketDapObjects = validateSTPacketDapObjectsFactory(
      dap.object.validateDapObject,
      findDuplicatedDapObjects,
    );

    this.validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketDapContracts,
      validateSTPacketDapObjects,
    );

    this.factory = new STPacketFactory(
      dap.getDataProvider(),
      this.validateSTPacket,
      createDapContract,
    );
  }

  /**
   * Create ST Packet
   *
   * @param {DapContract|Array} items
   * @return {STPacket}
   */
  create(items) {
    const dapContract = this.dap.getDapContract();

    if (!dapContract) {
      throw new MissingOptionError('dapContract');
    }

    return this.factory.create(dapContract.getId(), items);
  }

  /**
   *
   * @param {Object} rawSTPacket
   * @return {Promise<STPacket>}
   */
  async createFromObject(rawSTPacket) {
    return this.getFactory().createFromObject(rawSTPacket);
  }

  /**
   * Unserialize ST Packet
   *
   * @param {Buffer|string} payload
   * @return {Promise<STPacket>}
   */
  async createFromSerialized(payload) {
    return this.getFactory().createFromSerialized(payload);
  }

  /**
   *
   * @param {STPacket|Object} stPacket
   * @return {ValidationResult}
   */
  validate(stPacket) {
    const dapContract = this.dap.getDapContract();

    if (!dapContract) {
      throw new MissingOptionError('dapContract');
    }

    return this.validateSTPacket(stPacket, dapContract);
  }

  /**
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   * @return {Promise<ValidationResult>}
   */
  async verify(stPacket, stateTransition) {
    if (!this.dap.getDataProvider()) {
      throw new MissingOptionError('dataProvider');
    }

    const verifySTPacket = this.createVerifySTPacket();

    return verifySTPacket(stPacket, stateTransition);
  }

  /**
   * @private
   * @return {verifySTPacket}
   */
  createVerifySTPacket() {
    const verifyDapContract = verifyDapContractFactory(
      this.dap.getDataProvider(),
    );

    const fetchDapObjectsByObjects = fetchDapObjectsByObjectsFactory(
      this.dap.getDataProvider(),
    );

    const verifyDapObjects = verifyDapObjectsFactory(fetchDapObjectsByObjects);

    return verifySTPacketFactory(
      verifyDapContract,
      verifyDapObjects,
      this.dap.getDataProvider(),
    );
  }

  /**
   * @private
   * @return {STPacketFactory}
   */
  getFactory() {
    if (!this.dap.getDataProvider()) {
      throw new MissingOptionError('dataProvider');
    }

    this.factory.setDataProvider(this.dap.getDataProvider());

    return this.factory;
  }
}

module.exports = STPacketFacade;

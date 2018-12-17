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

    const verifyDapContract = verifyDapContractFactory(dap.getDataProvider());

    const fetchDapObjectsByObjects = fetchDapObjectsByObjectsFactory(dap.getDataProvider());
    const verifyDapObjects = verifyDapObjectsFactory(fetchDapObjectsByObjects);

    this.verifySTPacket = verifySTPacketFactory(
      verifyDapContract,
      verifyDapObjects,
      dap.getDataProvider(),
    );

    this.factory = new STPacketFactory(
      dap.getUserId(),
      dap.getDataProvider(),
      this.validateSTPacket,
      createDapContract,
    );
  }

  /**
   * Update dependencies
   *
   * @param {DashApplicationProtocol} dap
   */
  updateDependencies(dap) {
    this.factory.setUserId(dap.getUserId());
    this.factory.setDataProvider(dap.getDataProvider());
  }

  /**
   * Create ST Packet
   *
   * @param {DapContract|Array} items
   * @return {STPacket}
   */
  create(items) {
    return this.factory.create(this.dap.getDapContractId(), items);
  }

  /**
   *
   * @param {Object} object
   * @return {STPacket}
   */
  createFromObject(object) {
    return this.factory.createFromObject(object);
  }

  /**
   * Unserialize ST Packet
   *
   * @param {Buffer|string} payload
   * @return {STPacket}
   */
  createFromSerialized(payload) {
    return this.factory.createFromSerialized(payload);
  }

  /**
   *
   * @param {STPacket|Object} stPacket
   * @return {Object[]|*}
   */
  validate(stPacket) {
    return this.validateSTPacket(stPacket, this.dap.getDapContract());
  }

  /**
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   * @return {ValidationResult}
   */
  verify(stPacket, stateTransition) {
    return this.verifySTPacket(stPacket, stateTransition);
  }
}

module.exports = STPacketFacade;

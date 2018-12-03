const validateSTPacketStructureFactory = require('./validation/validateSTPacketStructureFactory');
const validateSTPacketFactory = require('./validation/validateSTPacketFactory');

const verifySTPacketFactory = require('./verification/verifySTPacketFactory');
const verifyDapContract = require('./verification/verifyDapContract');
const verifyDapObjectsFactory = require('./verification/verifyDapObjectsFactory');

const findDuplicatedPrimaryKeyAndType = require('./verification/findDuplicatedPrimaryKeyAndType');

const STPacketFactory = require('./STPacketFactory');

class STPacketFacade {
  /**
   * @param {DashApplicationProtocol} dap
   * @param {JsonSchemaValidator} validator
   */
  constructor(dap, validator) {
    this.dap = dap;

    this.validateSTPacket = validateSTPacketFactory(
      validator,
      dap.object.validateDapObject,
      dap.contract.validateDapContract,
    );

    this.factory = new STPacketFactory(
      dap.getUserId(),
      dap.getDataProvider(),
      validateSTPacketStructureFactory(validator),
      dap.contract.factory,
      dap.object.validateDapObject,
    );

    const verifyDapObjects = verifyDapObjectsFactory(findDuplicatedPrimaryKeyAndType);

    this.verifySTPacket = verifySTPacketFactory(verifyDapContract, verifyDapObjects);
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
   * @return {VerificationResult}
   */
  verify(stPacket, stateTransition) {
    return this.verifySTPacket(stPacket, stateTransition, this.dap.getDataProvider());
  }
}

module.exports = STPacketFacade;

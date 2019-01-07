const validateSTPacketFactory = require('./validation/validateSTPacketFactory');

const validateSTPacketDPContractsFactory = require('./validation/validateSTPacketDPContractsFactory');
const validateSTPacketDPObjectsFactory = require('./validation/validateSTPacketDPObjectsFactory');

const findDuplicatedDPObjects = require('./validation/findDuplicatedDPObjects');
const createDPContract = require('../contract/createDPContract');

const verifySTPacketFactory = require('./verification/verifySTPacketFactory');
const verifyDPContractFactory = require('./verification/verifyDPContractFactory');
const verifyDPObjectsFactory = require('./verification/verifyDPObjectsFactory');
const fetchDPObjectsByObjectsFactory = require('./verification/fetchDPObjectsByObjectsFactory');

const STPacketFactory = require('./STPacketFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class STPacketFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {JsonSchemaValidator} validator
   */
  constructor(dpp, validator) {
    this.dpp = dpp;

    const validateSTPacketDPContracts = validateSTPacketDPContractsFactory(
      dpp.contract.validateDPContract,
      createDPContract,
    );

    const validateSTPacketDPObjects = validateSTPacketDPObjectsFactory(
      dpp.object.validateDPObject,
      findDuplicatedDPObjects,
    );

    this.validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketDPContracts,
      validateSTPacketDPObjects,
    );

    this.factory = new STPacketFactory(
      dpp.getDataProvider(),
      this.validateSTPacket,
      createDPContract,
    );
  }

  /**
   * Create ST Packet
   *
   * @param {DPContract|Array} items
   * @return {STPacket}
   */
  create(items) {
    const dpContract = this.dpp.getDPContract();

    if (!dpContract) {
      throw new MissingOptionError(
        'dpContract',
        'Can\'t create ST Packet because DP Contract is not set, use setDPContract method',
      );
    }

    return this.factory.create(dpContract.getId(), items);
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
    const dpContract = this.dpp.getDPContract();

    if (!dpContract) {
      throw new MissingOptionError(
        'dpContract',
        'Can\'t validate ST Packet because DP Contract is not set, use setDPContract method',
      );
    }

    return this.validateSTPacket(stPacket, dpContract);
  }

  /**
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   * @return {Promise<ValidationResult>}
   */
  async verify(stPacket, stateTransition) {
    if (!this.dpp.getDataProvider()) {
      throw new MissingOptionError(
        'dataProvider',
        'Can\'t verify ST Packer because Data Provider is not set, use setDataProvider method',
      );
    }

    const verifySTPacket = this.createVerifySTPacket();

    return verifySTPacket(stPacket, stateTransition);
  }

  /**
   * @private
   * @return {verifySTPacket}
   */
  createVerifySTPacket() {
    const verifyDPContract = verifyDPContractFactory(
      this.dpp.getDataProvider(),
    );

    const fetchDPObjectsByObjects = fetchDPObjectsByObjectsFactory(
      this.dpp.getDataProvider(),
    );

    const verifyDPObjects = verifyDPObjectsFactory(fetchDPObjectsByObjects);

    return verifySTPacketFactory(
      verifyDPContract,
      verifyDPObjects,
      this.dpp.getDataProvider(),
    );
  }

  /**
   * @private
   * @return {STPacketFactory}
   */
  getFactory() {
    if (!this.dpp.getDataProvider()) {
      throw new MissingOptionError(
        'dataProvider',
        'Can\'t create ST Packer because Data Provider is not set, use setDataProvider method',
      );
    }

    this.factory.setDataProvider(this.dpp.getDataProvider());

    return this.factory;
  }
}

module.exports = STPacketFacade;

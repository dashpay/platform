const serializer = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketStructureError = require('./errors/InvalidSTPacketStructureError');

const DapObjectFactory = require('../dapObject/DapObjectFactory');

const DapContract = require('../dapContract/DapContract');

class STPacketFactory {
  /**
   * @param {string} userId
   * @param {AbstractDataProvider} dataProvider
   * @param {validateSTPacketStructure} validateSTPacketStructure
   * @param {DapContractFactory} dapContractFactory
   * @param {validateDapObject} validateDapObject
   */
  constructor(
    userId,
    dataProvider,
    validateSTPacketStructure,
    dapContractFactory,
    validateDapObject,
  ) {
    this.userId = userId;
    this.dataProvider = dataProvider;
    this.validateSTPacketStructure = validateSTPacketStructure;

    this.dapContractFactory = dapContractFactory;
    this.validateDapObject = validateDapObject;
  }

  /**
   * Create ST Packet
   *
   * @param {string} contractId
   * @param {DapContract|Array} [items]
   * @return {STPacket}
   */
  create(contractId, items = undefined) {
    const stPacket = new STPacket(contractId);

    if (items instanceof DapContract) {
      stPacket.addDapObject(items);
    }

    if (Array.isArray(items)) {
      stPacket.setDapObjects(items);
    }

    return stPacket;
  }

  /**
   *
   * @param {Object} object
   * @return {STPacket}
   */
  createFromObject(object) {
    const errors = this.validateSTPacketStructure(object);

    if (errors.length) {
      throw new InvalidSTPacketStructureError(errors, object);
    }

    const stPacket = this.create(object.contractId);

    stPacket.setItemsMerkleRoot(object.itemsMerkleRoot);
    stPacket.setItemsHash(object.itemsHash);

    if (object.contracts.length) {
      const dapContract = this.dapContractFactory.createFromObject(object.contracts[0]);

      stPacket.setDapContract(dapContract);
    }

    if (object.objects.length) {
      const dapContract = this.dataProvider.fetchDapContract(object.contractId);

      const dapObjectFactory = new DapObjectFactory(
        this.userId,
        dapContract,
        this.validateDapObject,
      );

      // eslint-disable-next-line arrow-body-style
      const dapObjects = object.objects.map((dapObject) => {
        return dapObjectFactory.createFromObject(dapObject);
      });

      stPacket.setDapObjects(dapObjects);
    }

    return stPacket;
  }

  /**
   * Unserialize ST Packet
   *
   * @param {Buffer|string} payload
   * @return {STPacket}
   */
  createFromSerialized(payload) {
    const object = serializer.decode(payload);

    return this.createFromObject(object);
  }

  /**
   * Set User ID
   *
   * @param {string} userId
   * @return {STPacketFactory}
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
   * Set Data Provider
   *
   * @param {AbstractDataProvider} dataProvider
   * @return {STPacketFactory}
   */
  setDataProvider(dataProvider) {
    this.dataProvider = dataProvider;

    return this;
  }

  /**
   * Get Data Provider
   *
   * @return {AbstractDataProvider}
   */
  getDataProvider() {
    return this.dataProvider;
  }
}

module.exports = STPacketFactory;

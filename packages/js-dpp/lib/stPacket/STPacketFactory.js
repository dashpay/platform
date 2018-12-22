const serializer = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketError = require('./errors/InvalidSTPacketError');

const DapContract = require('../dapContract/DapContract');
const DapObject = require('../dapObject/DapObject');

class STPacketFactory {
  /**
   * @param {string} userId
   * @param {AbstractDataProvider} dataProvider
   * @param {validateSTPacket} validateSTPacket
   * @param {createDapContract} createDapContract
   */
  constructor(
    userId,
    dataProvider,
    validateSTPacket,
    createDapContract,
  ) {
    this.userId = userId;
    this.dataProvider = dataProvider;
    this.validateSTPacket = validateSTPacket;
    this.createDapContract = createDapContract;
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
    // TODO We don't need contract if there are no objects in STPacket
    // TODO Check if contractId is present

    const dapContract = this.dataProvider.fetchDapContract(object.contractId);

    // TODO Return error if contract is not present

    const result = this.validateSTPacket(object, dapContract);

    if (!result.isValid()) {
      throw new InvalidSTPacketError(result.getErrors(), object);
    }

    const stPacket = this.create(object.contractId);

    stPacket.setItemsMerkleRoot(object.itemsMerkleRoot);
    stPacket.setItemsHash(object.itemsHash);

    if (object.contracts.length) {
      const packetDapContract = this.createDapContract(object.contracts[0]);

      stPacket.setDapContract(packetDapContract);
    }

    if (object.objects.length) {
      const dapObjects = object.objects.map(rawDapObject => new DapObject(rawDapObject));

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

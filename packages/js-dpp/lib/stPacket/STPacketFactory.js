const { decode } = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketError = require('./errors/InvalidSTPacketError');
const InvalidSTPacketContractIdError = require('../errors/InvalidSTPacketContractIdError');

const DapObject = require('../dapObject/DapObject');

class STPacketFactory {
  /**
   * @param {DataProvider} dataProvider
   * @param {validateSTPacket} validateSTPacket
   * @param {createDapContract} createDapContract
   */
  constructor(
    dataProvider,
    validateSTPacket,
    createDapContract,
  ) {
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
    return new STPacket(contractId, items);
  }

  /**
   * Create ST Packet from plain object
   *
   * @param {Object} object
   * @return {Promise<STPacket>}
   */
  async createFromObject(object) {
    let dapContract;
    if (object.contractId && Array.isArray(object.objects) && object.objects.length > 0) {
      dapContract = await this.dataProvider.fetchDapContract(object.contractId);

      if (!dapContract) {
        const error = new InvalidSTPacketContractIdError(object.contractId, dapContract);

        throw new InvalidSTPacketError([error], object);
      }
    }

    const result = this.validateSTPacket(object, dapContract);

    if (!result.isValid()) {
      throw new InvalidSTPacketError(result.getErrors(), object);
    }

    const stPacket = this.create(object.contractId);

    stPacket.setItemsMerkleRoot(object.itemsMerkleRoot);
    stPacket.setItemsHash(object.itemsHash);

    if (object.contracts.length > 0) {
      const packetDapContract = this.createDapContract(object.contracts[0]);

      stPacket.setDapContract(packetDapContract);
    }

    if (dapContract && object.objects.length > 0) {
      const dapObjects = object.objects.map(rawDapObject => new DapObject(rawDapObject));

      stPacket.setDapObjects(dapObjects);
    }

    return stPacket;
  }

  /**
   * Unserialize ST Packet
   *
   * @param {Buffer|string} payload
   * @return {Promise<STPacket>}
   */
  async createFromSerialized(payload) {
    const object = decode(payload);

    return this.createFromObject(object);
  }

  /**
   * Set Data Provider
   *
   * @param {DataProvider} dataProvider
   * @return {STPacketFactory}
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

module.exports = STPacketFactory;

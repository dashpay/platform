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
   * @param {Object} rawSTPacket
   * @return {Promise<STPacket>}
   */
  async createFromObject(rawSTPacket) {
    let dapContract;

    const areDapObjectsPresent = rawSTPacket.contractId
      && Array.isArray(rawSTPacket.objects)
      && rawSTPacket.objects.length > 0;

    if (areDapObjectsPresent) {
      dapContract = await this.dataProvider.fetchDapContract(rawSTPacket.contractId);

      if (!dapContract) {
        const error = new InvalidSTPacketContractIdError(rawSTPacket.contractId, dapContract);

        throw new InvalidSTPacketError([error], rawSTPacket);
      }
    }

    const result = this.validateSTPacket(rawSTPacket, dapContract);

    if (!result.isValid()) {
      throw new InvalidSTPacketError(result.getErrors(), rawSTPacket);
    }

    const stPacket = this.create(rawSTPacket.contractId);

    if (rawSTPacket.contracts.length > 0) {
      const packetDapContract = this.createDapContract(rawSTPacket.contracts[0]);

      stPacket.setDapContract(packetDapContract);
    }

    if (dapContract && rawSTPacket.objects.length > 0) {
      const dapObjects = rawSTPacket.objects.map(rawDapObject => new DapObject(rawDapObject));

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
    const rawSTPacket = decode(payload);

    return this.createFromObject(rawSTPacket);
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

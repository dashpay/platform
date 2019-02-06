const { decode } = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketError = require('./errors/InvalidSTPacketError');
const DPContractNotPresentError = require('../errors/DPContractNotPresentError');

const DPObject = require('../object/DPObject');

class STPacketFactory {
  /**
   * @param {DataProvider} dataProvider
   * @param {validateSTPacket} validateSTPacket
   * @param {createDPContract} createDPContract
   */
  constructor(
    dataProvider,
    validateSTPacket,
    createDPContract,
  ) {
    this.dataProvider = dataProvider;
    this.validateSTPacket = validateSTPacket;
    this.createDPContract = createDPContract;
  }

  /**
   * Create ST Packet
   *
   * @param {string} contractId
   * @param {DPContract|Array} [items]
   * @return {STPacket}
   */
  create(contractId, items = undefined) {
    return new STPacket(contractId, items);
  }

  /**
   * Create ST Packet from plain object
   *
   * @param {Object} rawSTPacket
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<STPacket>}
   */
  async createFromObject(rawSTPacket, options = { skipValidation: false }) {
    if (!options.skipValidation) {
      let dpContract;

      const areDPObjectsPresent = rawSTPacket.contractId
        && Array.isArray(rawSTPacket.objects)
        && rawSTPacket.objects.length > 0;

      if (areDPObjectsPresent) {
        dpContract = await this.dataProvider.fetchDPContract(rawSTPacket.contractId);

        if (!dpContract) {
          const error = new DPContractNotPresentError(rawSTPacket.contractId);

          throw new InvalidSTPacketError([error], rawSTPacket);
        }
      }

      const result = this.validateSTPacket(rawSTPacket, dpContract);

      if (!result.isValid()) {
        throw new InvalidSTPacketError(result.getErrors(), rawSTPacket);
      }
    }

    const stPacket = this.create(rawSTPacket.contractId);

    if (rawSTPacket.contracts.length > 0) {
      const packetDPContract = this.createDPContract(rawSTPacket.contracts[0]);

      stPacket.setDPContract(packetDPContract);
    }

    if (rawSTPacket.objects.length > 0) {
      const dpObjects = rawSTPacket.objects.map(rawDPObject => new DPObject(rawDPObject));

      stPacket.setDPObjects(dpObjects);
    }

    return stPacket;
  }

  /**
   * Unserialize ST Packet
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<STPacket>}
   */
  async createFromSerialized(payload, options = { skipValidation: false }) {
    const rawSTPacket = decode(payload);

    return this.createFromObject(rawSTPacket, options);
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

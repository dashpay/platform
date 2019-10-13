const { decode } = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketError = require('./errors/InvalidSTPacketError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');

const Document = require('../document/Document');

class STPacketFactory {
  /**
   * @param {DataProvider} dataProvider
   * @param {validateSTPacket} validateSTPacket
   * @param {createDataContract} createDataContract
   */
  constructor(
    dataProvider,
    validateSTPacket,
    createDataContract,
  ) {
    this.dataProvider = dataProvider;
    this.validateSTPacket = validateSTPacket;
    this.createContract = createDataContract;
  }

  /**
   * Create ST Packet
   *
   * @param {string} contractId
   * @param {DataContract|Document[]} [items]
   * @return {STPacket}
   */
  create(contractId, items = undefined) {
    return new STPacket(contractId, items);
  }

  /**
   * Create ST Packet from plain object
   *
   * @param {RawSTPacket} rawSTPacket
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<STPacket>}
   */
  async createFromObject(rawSTPacket, options = { skipValidation: false }) {
    if (!options.skipValidation) {
      let contract;

      const areDocumentsPresent = rawSTPacket.contractId
        && Array.isArray(rawSTPacket.documents)
        && rawSTPacket.documents.length > 0;

      if (areDocumentsPresent) {
        contract = await this.dataProvider.fetchContract(rawSTPacket.contractId);

        if (!contract) {
          const error = new DataContractNotPresentError(rawSTPacket.contractId);

          throw new InvalidSTPacketError([error], rawSTPacket);
        }
      }

      const result = this.validateSTPacket(rawSTPacket, contract);

      if (!result.isValid()) {
        throw new InvalidSTPacketError(result.getErrors(), rawSTPacket);
      }
    }

    const stPacket = this.create(rawSTPacket.contractId);

    if (rawSTPacket.contracts.length > 0) {
      const packetContract = this.createContract(rawSTPacket.contracts[0]);

      stPacket.setContract(packetContract);
    }

    if (rawSTPacket.documents.length > 0) {
      const documents = rawSTPacket.documents.map(rawDocument => new Document(rawDocument));

      stPacket.setDocuments(documents);
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

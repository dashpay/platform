const serializer = require('../util/serializer');

const STPacket = require('./STPacket');

const InvalidSTPacketStructureError = require('./errors/InvalidSTPacketStructureError');

const DapObjectFactory = require('../dapObject/DapObjectFactory');
const DapContractFactory = require('../dapContract/DapContractFactory');

class STPacketFactory {
  /**
   * @param {string} blockchainUserId
   * @param {AbstractDataProvider} dataProvider
   * @param {validateSTPacketStructure} validateSTPacketStructure
   */
  constructor(blockchainUserId, dataProvider, validateSTPacketStructure, validateDapContract, validateDapObject) {
    this.blockchainUserId = blockchainUserId;
    this.dataProvider = dataProvider;
    this.validateSTPacketStructure = validateSTPacketStructure;

    this.dapContractFactory = new DapContractFactory(validateDapContract);
    this.validateDapObject = validateDapObject;
  }

  /**
   * Create ST Packet
   *
   * @param {string} contractId
   * @return {STPacket}
   */
  create(contractId) {
    return new STPacket(contractId);
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

    const stPacket = new STPacket(object.contractId);

    stPacket.setItemsMerkleRoot(object.itemsMerkleRoot);
    stPacket.setItemsHash(object.itemsHash);

    if (object.contracts.length) {
      const dapContract = this.dapContractFactory.createFromObject(object.contracts[0]);

      stPacket.setDapContract(dapContract);
    }

    if (object.objects.length) {
      const dapContract = this.dataProvider.fetchDapContract(object.contractId);

      const dapObjectFactory = new DapObjectFactory(
        dapContract,
        this.blockchainUserId,
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
}

module.exports = STPacketFactory;

const validateSTPacketFactory = require('./validation/validateSTPacketFactory');

const validateSTPacketDPContractsFactory = require('./validation/validateSTPacketDPContractsFactory');
const validateSTPacketDocumentsFactory = require('./validation/validateSTPacketDocumentsFactory');

const findDuplicateDocuments = require('./validation/findDuplicateDocuments');
const findDuplicateDocumentsByIndices = require('./validation/findDuplicateDocumentsByIndices');
const createDPContract = require('../contract/createDPContract');

const verifySTPacketFactory = require('./verification/verifySTPacketFactory');
const verifyDPContract = require('./verification/verifyDPContract');
const verifyDocumentsFactory = require('./verification/verifyDocumentsFactory');
const verifyDocumentsUniquenessByIndicesFactory = require('./verification/verifyDocumentsUniquenessByIndicesFactory');
const fetchDocumentsByDocumentsFactory = require('./verification/fetchDocumentsByDocumentsFactory');

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
    );

    const validateSTPacketDocuments = validateSTPacketDocumentsFactory(
      dpp.document.validateDocument,
      findDuplicateDocuments,
      findDuplicateDocumentsByIndices,
    );

    this.validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketDPContracts,
      validateSTPacketDocuments,
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
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<STPacket>}
   */
  async createFromObject(rawSTPacket, options = { skipValidation: false }) {
    return this.getFactory().createFromObject(rawSTPacket, options);
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
    return this.getFactory().createFromSerialized(payload, options);
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
    const fetchDocumentsByDocuments = fetchDocumentsByDocumentsFactory(
      this.dpp.getDataProvider(),
    );

    const verifyDocumentsUniquenessByIndices = verifyDocumentsUniquenessByIndicesFactory(
      fetchDocumentsByDocuments,
      this.dpp.getDataProvider(),
    );

    const verifyDocuments = verifyDocumentsFactory(
      fetchDocumentsByDocuments,
      verifyDocumentsUniquenessByIndices,
    );

    return verifySTPacketFactory(
      verifyDPContract,
      verifyDocuments,
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

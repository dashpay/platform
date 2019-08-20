const validateSTPacketFactory = require('./validation/validateSTPacketFactory');

const validateSTPacketContractsFactory = require('./validation/validateSTPacketContractsFactory');
const validateSTPacketDocumentsFactory = require('./validation/validateSTPacketDocumentsFactory');

const findDuplicateDocuments = require('./validation/findDuplicateDocuments');
const findDuplicateDocumentsByIndices = require('./validation/findDuplicateDocumentsByIndices');
const createContract = require('../contract/createContract');

const verifySTPacketFactory = require('./verification/verifySTPacketFactory');
const verifyContract = require('./verification/verifyContract');
const executeDataTriggersFactory = require('./verification/executeDataTriggersFactory');
const verifyDocumentsFactory = require('./verification/verifyDocumentsFactory');
const verifyDocumentsUniquenessByIndicesFactory = require('./verification/verifyDocumentsUniquenessByIndicesFactory');
const fetchDocumentsByDocumentsFactory = require('./verification/fetchDocumentsByDocumentsFactory');

const getDataTriggersFactory = require('../dataTrigger/getDataTriggersFactory');

const STPacketFactory = require('./STPacketFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class STPacketFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {JsonSchemaValidator} validator
   */
  constructor(dpp, validator) {
    this.dpp = dpp;

    const validateSTPacketContracts = validateSTPacketContractsFactory(
      dpp.contract.validateContract,
    );

    const validateSTPacketDocuments = validateSTPacketDocumentsFactory(
      dpp.document.validateDocument,
      findDuplicateDocuments,
      findDuplicateDocumentsByIndices,
    );

    this.validateSTPacket = validateSTPacketFactory(
      validator,
      validateSTPacketContracts,
      validateSTPacketDocuments,
    );

    this.factory = new STPacketFactory(
      dpp.getDataProvider(),
      this.validateSTPacket,
      createContract,
    );
  }

  /**
   * Create ST Packet
   *
   * @param {Contract|Array} items
   * @return {STPacket}
   */
  create(items) {
    const contract = this.dpp.getContract();

    if (!contract) {
      throw new MissingOptionError(
        'contract',
        'Can\'t create ST Packet because Contract is not set, use setContract method',
      );
    }

    return this.factory.create(contract.getId(), items);
  }

  /**
   *
   * @param {RawSTPacket} rawSTPacket
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
   * Validate ST Packet
   *
   * @param {STPacket|RawSTPacket} stPacket
   * @return {ValidationResult}
   */
  validate(stPacket) {
    const contract = this.dpp.getContract();

    if (!contract) {
      throw new MissingOptionError(
        'contract',
        'Can\'t validate ST Packet because Contract is not set, use setContract method',
      );
    }

    return this.validateSTPacket(stPacket, contract);
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

    const getDataTriggers = getDataTriggersFactory();

    const executeDataTriggers = executeDataTriggersFactory(
      getDataTriggers,
    );

    return verifySTPacketFactory(
      verifyContract,
      verifyDocuments,
      this.dpp.getDataProvider(),
      executeDataTriggers,
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

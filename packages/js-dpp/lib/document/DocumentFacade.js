const enrichDataContractWithBaseSchema = require('../dataContract/enrichDataContractWithBaseSchema');
const validateDocumentFactory = require('./validateDocumentFactory');
const fetchAndValidateDataContractFactory = require('./fetchAndValidateDataContractFactory');

const DocumentFactory = require('./DocumentFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class DocumentFacade {
  /**
   * @param {StateRepository} stateRepository
   * @param {JsonSchemaValidator} validator
   */
  constructor(stateRepository, validator) {
    this.stateRepository = stateRepository;

    this.validateDocument = validateDocumentFactory(
      validator,
      enrichDataContractWithBaseSchema,
    );

    this.fetchAndValidateDataContract = fetchAndValidateDataContractFactory(
      stateRepository,
    );

    this.factory = new DocumentFactory(
      this.validateDocument,
      this.fetchAndValidateDataContract,
    );
  }

  /**
   * Create Document
   *
   * @param {DataContract} dataContract
   * @param {string} ownerId
   * @param {string} type
   * @param {Object} [data]
   * @return {Document}
   */
  create(dataContract, ownerId, type, data = {}) {
    return this.factory.create(dataContract, ownerId, type, data);
  }

  /**
   * Create Document from plain object
   *
   * @param {RawDocument} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.action]
   * @return {Document}
   */
  async createFromObject(rawDocument, options = {}) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create Document because State Repository is not set in'
        + ' DashPlatformProtocol options',
      );
    }

    return this.factory.createFromObject(rawDocument, options);
  }

  /**
   * Create Document from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.action]
   * @return {Promise<Document>}
   */
  async createFromSerialized(payload, options = { }) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create Document because State Repository is not set in'
        + ' DashPlatformProtocol options',
      );
    }

    return this.factory.createFromSerialized(payload, options);
  }

  /**
   * Create Documents State Transition
   *
   * @param {Object} documents
   * @param {Document[]} [documents.create]
   * @param {Document[]} [documents.replace]
   * @param {Document[]} [documents.delete]
   *
   * @return {DocumentsBatchTransition}
   */
  createStateTransition(documents) {
    return this.factory.createStateTransition(documents);
  }

  /**
   * Validate document
   *
   * @param {Document|RawDocument} document
   * @param {Object} options
   * @param {number} [options.action=1]
   * @return {ValidationResult}
   */
  async validate(document, options = {}) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate Document because State Repository is not set in'
        + ' DashPlatformProtocol options',
      );
    }

    const result = await this.fetchAndValidateDataContract(document);

    if (!result.isValid()) {
      return result;
    }

    const dataContract = result.getData();

    return this.validateDocument(document, dataContract, options);
  }
}

module.exports = DocumentFacade;

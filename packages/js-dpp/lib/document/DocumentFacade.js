const enrichDataContractWithBaseSchema = require('../dataContract/enrichDataContractWithBaseSchema');
const validateDocumentFactory = require('./validateDocumentFactory');
const fetchAndValidateDataContractFactory = require('./fetchAndValidateDataContractFactory');

const Document = require('./Document');
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
   * @param {Buffer} ownerId
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
   * @return {Promise<Document>}
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
   * Create Document from buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.action]
   * @return {Promise<Document>}
   */
  async createFromBuffer(buffer, options = { }) {
    if (!this.stateRepository && !options.skipValidation) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t create Document because State Repository is not set in'
        + ' DashPlatformProtocol options',
      );
    }

    return this.factory.createFromBuffer(buffer, options);
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
   * @return {<ValidationResult>}
   */
  async validate(document) {
    if (!this.stateRepository) {
      throw new MissingOptionError(
        'stateRepository',
        'Can\'t validate Document because State Repository is not set in'
        + ' DashPlatformProtocol options',
      );
    }

    let rawDocument;
    if (document instanceof Document) {
      rawDocument = document.toObject();
    } else {
      rawDocument = document;
    }

    const result = await this.fetchAndValidateDataContract(rawDocument);

    if (!result.isValid()) {
      return result;
    }

    const dataContract = result.getData();

    return this.validateDocument(rawDocument, dataContract);
  }
}

module.exports = DocumentFacade;

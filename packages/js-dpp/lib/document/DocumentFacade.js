const enrichDataContractWithBaseDocument = require('./enrichDataContractWithBaseDocument');
const validateDocumentFactory = require('./validateDocumentFactory');

const DocumentFactory = require('./DocumentFactory');

const MissingOptionError = require('../errors/MissingOptionError');

class DocumentFacade {
  /**
   *
   * @param {DashPlatformProtocol} dpp
   * @param {JsonSchemaValidator} validator
   */
  constructor(dpp, validator) {
    this.dpp = dpp;

    this.validateDocument = validateDocumentFactory(
      validator,
      enrichDataContractWithBaseDocument,
    );

    this.factory = new DocumentFactory(
      dpp.getUserId(),
      dpp.getDataContract(),
      this.validateDocument,
    );
  }

  /**
   * Create Document
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {Document}
   */
  create(type, data = {}) {
    return this.getFactory().create(type, data);
  }

  /**
   * Create Document from plain object
   *
   * @param {RawDocument} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.allowMeta=true]
   * @param {boolean} [options.action]
   * @return {Document}
   */
  createFromObject(rawDocument, options = { }) {
    return this.getFactory().createFromObject(rawDocument, options);
  }

  /**
   * Create Document from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.allowMeta=true]
   * @param {boolean} [options.action]
   * @return {Document}
   */
  createFromSerialized(payload, options = { }) {
    return this.getFactory().createFromSerialized(payload, options);
  }

  /**
   * Validate document
   *
   * @param {Document|RawDocument} document
   * @return {ValidationResult}
   */
  validate(document) {
    return this.validateDocument(document, this.dpp.getDataContract());
  }

  /**
   * @private
   * @return {DocumentFactory}
   */
  getFactory() {
    if (!this.dpp.getUserId()) {
      throw new MissingOptionError(
        'userId',
        'Can\'t create Document because User ID is not set, use setUserId method',
      );
    }

    if (!this.dpp.getDataContract()) {
      throw new MissingOptionError(
        'contract',
        'Can\'t create Document because Data Contract is not set, use setDataContract method',
      );
    }

    this.factory.setUserId(this.dpp.getUserId());
    this.factory.setDataContract(this.dpp.getDataContract());

    return this.factory;
  }
}

module.exports = DocumentFacade;

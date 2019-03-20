const enrichDPContractWithBaseDocument = require('./enrichDPContractWithBaseDocument');
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
      enrichDPContractWithBaseDocument,
    );

    this.factory = new DocumentFactory(
      dpp.getUserId(),
      dpp.getDPContract(),
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
   * @param {Object} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Document}
   */
  createFromObject(rawDocument, options = { skipValidation: false }) {
    return this.getFactory().createFromObject(rawDocument, options);
  }

  /**
   * Create Document from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Document}
   */
  createFromSerialized(payload, options = { skipValidation: false }) {
    return this.getFactory().createFromSerialized(payload, options);
  }

  /**
   *
   * @param {Object|Document} document
   * @return {ValidationResult}
   */
  validate(document) {
    return this.validateDocument(document, this.dpp.getDPContract());
  }

  /**
   * @private
   * @return {DocumentFactory}
   */
  getFactory() {
    if (!this.dpp.getUserId()) {
      throw new MissingOptionError(
        'userId',
        'Can\'t create packet because User ID is not set, use setUserId method',
      );
    }

    if (!this.dpp.getDPContract()) {
      throw new MissingOptionError(
        'dpContract',
        'Can\'t create Document because DP Contract is not set, use setDPContract method',
      );
    }

    this.factory.setUserId(this.dpp.getUserId());
    this.factory.setDPContract(this.dpp.getDPContract());

    return this.factory;
  }
}

module.exports = DocumentFacade;

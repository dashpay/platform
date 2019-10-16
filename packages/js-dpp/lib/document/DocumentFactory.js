const Document = require('./Document');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');

const InvalidDocumentError = require('./errors/InvalidDocumentError');
const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DocumentFactory {
  /**
   * @param {string} userId
   * @param {DataContract} dataContract
   * @param {validateDocument} validateDocument
   */
  constructor(userId, dataContract, validateDocument) {
    this.userId = userId;
    this.dataContract = dataContract;
    this.validateDocument = validateDocument;
  }

  /**
   * Create Document
   *
   * @param {string} type
   * @param {Object} [data]
   * @return {Document}
   */
  create(type, data = {}) {
    if (!this.dataContract.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this.dataContract);
    }

    const rawDocument = {
      $type: type,
      $contractId: this.dataContract.getId(),
      $userId: this.userId,
      $entropy: entropy.generate(),
      $rev: Document.DEFAULTS.REVISION,
      ...data,
    };

    const document = new Document(rawDocument);

    document.setAction(Document.DEFAULTS.ACTION);

    return document;
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
  createFromObject(rawDocument, options = {}) {
    const opts = Object.assign({ skipValidation: false }, options);

    if (!opts.skipValidation) {
      const result = this.validateDocument(
        rawDocument,
        this.dataContract,
        opts,
      );

      if (!result.isValid()) {
        throw new InvalidDocumentError(result.getErrors(), rawDocument);
      }
    }

    return new Document(rawDocument);
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
    const rawDocument = decode(payload);

    return this.createFromObject(rawDocument, options);
  }

  /**
   * Set User ID
   *
   * @param userId
   * @return {DocumentFactory}
   */
  setUserId(userId) {
    this.userId = userId;

    return this;
  }

  /**
   * Get User ID
   *
   * @return {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Set Data Contract
   *
   * @param {DataContract} dataContract
   * @return {DocumentFactory}
   */
  setDataContract(dataContract) {
    this.dataContract = dataContract;

    return this;
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = DocumentFactory;

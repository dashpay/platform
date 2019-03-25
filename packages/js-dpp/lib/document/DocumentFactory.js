const Document = require('./Document');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');
const hash = require('../util/hash');

const InvalidDocumentError = require('./errors/InvalidDocumentError');
const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DocumentFactory {
  /**
   * @param {Contract} contract
   * @param {string} userId
   * @param {validateDocument} validateDocument
   */
  constructor(userId, contract, validateDocument) {
    this.userId = userId;
    this.contract = contract;
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
    if (!this.contract.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, this.contract);
    }

    const rawDocument = {
      $type: type,
      $scope: hash(this.contract.getId() + this.userId).toString('hex'),
      $scopeId: entropy.generate(),
      $action: Document.DEFAULTS.ACTION,
      $rev: Document.DEFAULTS.REVISION,
      ...data,
    };

    return new Document(rawDocument);
  }


  /**
   * Create Document from plain object
   *
   * @param {RawDocument} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Document}
   */
  createFromObject(rawDocument, options = { skipValidation: false }) {
    if (!options.skipValidation) {
      const result = this.validateDocument(rawDocument, this.contract);

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
   * @return {Document}
   */
  createFromSerialized(payload, options = { skipValidation: false }) {
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
   * Set Contract
   *
   * @param {Contract} contract
   * @return {DocumentFactory}
   */
  setContract(contract) {
    this.contract = contract;

    return this;
  }

  /**
   * Get Contract
   *
   * @return {Contract}
   */
  getContract() {
    return this.contract;
  }
}

module.exports = DocumentFactory;

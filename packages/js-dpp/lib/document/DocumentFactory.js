const Document = require('./Document');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');

const DocumentsStateTransition = require('./stateTransition/DocumentsStateTransition');

const InvalidDocumentError = require('./errors/InvalidDocumentError');
const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');

class DocumentFactory {
  /**
   * @param {validateDocument} validateDocument
   * @param {fetchAndValidateDataContract} fetchAndValidateDataContract
   */
  constructor(validateDocument, fetchAndValidateDataContract) {
    this.validateDocument = validateDocument;
    this.fetchAndValidateDataContract = fetchAndValidateDataContract;
  }

  /**
   * Create Document
   *
   * @param {DataContract} dataContract
   * @param {string} userId
   * @param {string} type
   * @param {Object} [data]
   * @return {Document}
   */
  create(dataContract, userId, type, data = {}) {
    if (!dataContract.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, dataContract);
    }

    const rawDocument = {
      $type: type,
      $contractId: dataContract.getId(),
      $userId: userId,
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
  async createFromObject(rawDocument, options = {}) {
    const opts = Object.assign({ skipValidation: false }, options);

    if (!opts.skipValidation) {
      const result = await this.fetchAndValidateDataContract(rawDocument);

      if (result.isValid()) {
        const dataContract = result.getData();

        result.merge(
          this.validateDocument(
            rawDocument,
            dataContract,
            opts,
          ),
        );
      }

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
   * @param {boolean} [options.action]
   * @return {Document}
   */
  createFromSerialized(payload, options = { }) {
    const rawDocument = decode(payload);

    return this.createFromObject(rawDocument, options);
  }

  /**
   * Create Documents State Transition
   *
   * @param {Document[]} documents
   * @return {DocumentsStateTransition}
   */
  createStateTransition(documents) {
    return new DocumentsStateTransition(documents);
  }
}

module.exports = DocumentFactory;

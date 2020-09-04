const Document = require('./Document');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');

const DocumentsBatchTransition = require('./stateTransition/DocumentsBatchTransition');

const AbstractDocumentTransition = require('./stateTransition/documentTransition/AbstractDocumentTransition');
const DocumentCreateTransition = require('./stateTransition/documentTransition/DocumentCreateTransition');

const InvalidActionNameError = require('./errors/InvalidActionNameError');
const NoDocumentsSuppliedError = require('./errors/NoDocumentsSuppliedError');
const MismatchOwnerIdsError = require('./errors/MismatchOwnerIdsError');
const InvalidInitialRevisionError = require('./errors/InvalidInitialRevisionError');

const InvalidDocumentError = require('./errors/InvalidDocumentError');
const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');

const generateDocumentId = require('./generateDocumentId');

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
   * @param {string} ownerId
   * @param {string} type
   * @param {Object} [data]
   * @return {Document}
   */
  create(dataContract, ownerId, type, data = {}) {
    if (!dataContract.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, dataContract);
    }

    const documentEntropy = entropy.generate();
    const dataContractId = dataContract.getId();

    const id = generateDocumentId(
      dataContractId,
      ownerId,
      type,
      documentEntropy,
    );

    const rawDocument = {
      $protocolVersion: Document.PROTOCOL_VERSION,
      $id: id,
      $type: type,
      $dataContractId: dataContractId,
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      ...data,
    };

    // We should set timestamps
    // Only if they are required by the contract
    const { required: documentRequiredFields } = dataContract.getDocumentSchema(type);

    const creationTime = new Date().getTime();

    if (documentRequiredFields
        && documentRequiredFields.includes('$createdAt')) {
      rawDocument.$createdAt = creationTime;
    }

    if (documentRequiredFields
      && documentRequiredFields.includes('$updatedAt')) {
      rawDocument.$updatedAt = creationTime;
    }

    const document = new Document(rawDocument, dataContract);

    document.setEntropy(documentEntropy);

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
    const dataContract = await this.validateDataContractForDocument(rawDocument, options);

    return new Document(rawDocument, dataContract);
  }

  /**
   * Create Document from JSON
   *
   * @param {RawDocument} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.action]
   * @return {Document}
   */
  async createFromJson(rawDocument, options = {}) {
    const dataContract = await this.validateDataContractForDocument(rawDocument, options);

    return Document.fromJSON(rawDocument, dataContract);
  }

  /**
   * @private
   *
   * @param {RawDocument} rawDocument
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @param {boolean} [options.action]
   *
   * @return {Promise<DataContract>}
   */
  async validateDataContractForDocument(rawDocument, options = {}) {
    const opts = { skipValidation: false, ...options };

    const result = await this.fetchAndValidateDataContract(rawDocument);

    if (!result.isValid()) {
      throw new InvalidDocumentError(result.getErrors(), rawDocument);
    }

    const dataContract = result.getData();

    if (!opts.skipValidation) {
      result.merge(
        this.validateDocument(
          rawDocument,
          dataContract,
          opts,
        ),
      );

      if (!result.isValid()) {
        throw new InvalidDocumentError(result.getErrors(), rawDocument);
      }
    }

    return dataContract;
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
    let rawDocument;
    try {
      rawDocument = decode(payload);
    } catch (error) {
      throw new InvalidDocumentError([
        new SerializedObjectParsingError(
          payload,
          error,
        ),
      ]);
    }

    return this.createFromObject(rawDocument, options);
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
    // Check no wrong actions were supplied
    const allowedKeys = Object.values(AbstractDocumentTransition.ACTION_NAMES);

    const actionKeys = Object.keys(documents);
    const filteredKeys = actionKeys
      .filter((key) => allowedKeys.indexOf(key) === -1);

    if (filteredKeys.length > 0) {
      throw new InvalidActionNameError(filteredKeys);
    }

    const documentsFlattened = actionKeys
      .reduce((all, t) => all.concat(documents[t]), []);

    if (documentsFlattened.length === 0) {
      throw new NoDocumentsSuppliedError();
    }

    // Check that documents are not mixed
    const [aDocument] = documentsFlattened;

    const ownerId = aDocument.getOwnerId();

    const mismatchedOwnerIdsLength = documentsFlattened
      .reduce((result, document) => {
        if (document.getOwnerId() !== ownerId) {
          // eslint-disable-next-line no-param-reassign
          result += 1;
        }

        return result;
      }, 0);

    if (mismatchedOwnerIdsLength > 0) {
      throw new MismatchOwnerIdsError(documentsFlattened);
    }

    // Convert documents to action transitions
    const {
      [AbstractDocumentTransition.ACTION_NAMES.CREATE]: createDocuments,
      [AbstractDocumentTransition.ACTION_NAMES.REPLACE]: replaceDocuments,
      [AbstractDocumentTransition.ACTION_NAMES.DELETE]: deleteDocuments,
    } = documents;

    const rawDocumentCreateTransitions = (createDocuments || [])
      .map((document) => {
        if (document.getRevision() !== DocumentCreateTransition.INITIAL_REVISION) {
          throw new InvalidInitialRevisionError(document);
        }

        const json = {
          $action: AbstractDocumentTransition.ACTIONS.CREATE,
          $id: document.getId(),
          $type: document.getType(),
          $dataContractId: document.getDataContractId(),
          $entropy: document.getEntropy(),
          ...document.getData(),
        };

        if (document.getCreatedAt()) {
          json.$createdAt = document.getCreatedAt().getTime();
        }

        if (document.getUpdatedAt()) {
          json.$updatedAt = document.getUpdatedAt().getTime();
        }

        return json;
      });

    const rawDocumentReplaceTransitions = (replaceDocuments || [])
      .map((document) => {
        const json = {
          $action: AbstractDocumentTransition.ACTIONS.REPLACE,
          $id: document.getId(),
          $type: document.getType(),
          $dataContractId: document.getDataContractId(),
          $revision: document.getRevision() + 1,
          ...document.getData(),
        };

        // If document have an originally set `updatedAt`
        // we should update it then
        if (document.getUpdatedAt()) {
          json.$updatedAt = new Date().getTime();
        }

        return json;
      });

    const rawDocumentDeleteTransitions = (deleteDocuments || [])
      .map((document) => ({
        $action: AbstractDocumentTransition.ACTIONS.DELETE,
        $id: document.getId(),
        $type: document.getType(),
        $dataContractId: document.getDataContractId(),
      }));

    const rawDocumentTransitions = rawDocumentCreateTransitions
      .concat(rawDocumentReplaceTransitions)
      .concat(rawDocumentDeleteTransitions);

    const dataContracts = documentsFlattened
      .map((document) => document.getDataContract());

    return new DocumentsBatchTransition({
      protocolVersion: Document.PROTOCOL_VERSION,
      ownerId,
      transitions: rawDocumentTransitions,
    }, dataContracts);
  }
}

module.exports = DocumentFactory;

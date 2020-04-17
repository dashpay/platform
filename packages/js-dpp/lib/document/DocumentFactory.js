const Document = require('./Document');

const { decode } = require('../util/serializer');
const entropy = require('../util/entropy');

const DocumentsBatchTransition = require('./stateTransition/DocumentsBatchTransition');

const AbstractDocumentTransition = require('./stateTransition/documentTransition/AbstractDocumentTransition');
const DocumentCreateTransition = require('./stateTransition/documentTransition/DocumentCreateTransition');

const InvalidActionNameError = require('./errors/InvalidActionNameError');
const NoDocumentsSuppliedError = require('./errors/NoDocumentsSuppliedError');
const MismatchContractIdsError = require('./errors/MismatchContractIdsError');
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
      $id: id,
      $type: type,
      $dataContractId: dataContractId,
      $ownerId: ownerId,
      $revision: DocumentCreateTransition.INITIAL_REVISION,
      ...data,
    };

    const document = new Document(rawDocument);

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
    const opts = { skipValidation: false, ...options };

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

    const contractId = aDocument.getDataContractId();
    const ownerId = aDocument.getOwnerId();

    const {
      mismatchedContractIdsLength,
      mismatchedOwnerIdsLength,
    } = documentsFlattened
      .reduce((result, document) => {
        if (document.getDataContractId() !== contractId) {
          // eslint-disable-next-line no-param-reassign
          result.mismatchedContractIdsLength += 1;
        }

        if (document.getOwnerId() !== ownerId) {
          // eslint-disable-next-line no-param-reassign
          result.mismatchedOwnerIdsLength += 1;
        }

        return result;
      }, { mismatchedContractIdsLength: 0, mismatchedOwnerIdsLength: 0 });

    if (mismatchedContractIdsLength > 0) {
      throw new MismatchContractIdsError(documentsFlattened);
    }

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

        return {
          $action: AbstractDocumentTransition.ACTIONS.CREATE,
          $id: document.getId(),
          $type: document.getType(),
          $dataContractId: document.getDataContractId(),
          $entropy: document.getEntropy(),
          ...document.getData(),
        };
      });

    const rawDocumentReplaceTransitions = (replaceDocuments || [])
      .map((document) => ({
        $action: AbstractDocumentTransition.ACTIONS.REPLACE,
        $id: document.getId(),
        $type: document.getType(),
        $dataContractId: document.getDataContractId(),
        $revision: document.getRevision() + 1,
        ...document.getData(),
      }));

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

    return new DocumentsBatchTransition({
      ownerId,
      transitions: rawDocumentTransitions,
    });
  }
}

module.exports = DocumentFactory;

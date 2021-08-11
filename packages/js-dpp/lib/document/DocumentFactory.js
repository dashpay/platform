const Document = require('./Document');

const { decode } = require('../util/serializer');
const generateEntropy = require('../util/generateEntropy');

const DocumentsBatchTransition = require('./stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const AbstractDocumentTransition = require('./stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');
const DocumentCreateTransition = require('./stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition');

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
   * @param {DashPlatformProtocol} dpp
   * @param {validateDocument} validateDocument
   * @param {fetchAndValidateDataContract} fetchAndValidateDataContract
   */
  constructor(dpp, validateDocument, fetchAndValidateDataContract) {
    this.dpp = dpp;
    this.validateDocument = validateDocument;
    this.fetchAndValidateDataContract = fetchAndValidateDataContract;
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
    if (!dataContract.isDocumentDefined(type)) {
      throw new InvalidDocumentTypeError(type, dataContract);
    }

    const documentEntropy = generateEntropy();
    const dataContractId = dataContract.getId();

    const id = generateDocumentId(
      dataContractId,
      ownerId,
      type,
      documentEntropy,
    );

    const rawDocument = {
      $protocolVersion: this.dpp.getProtocolVersion(),
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

    const result = this.validateDocument(
      rawDocument,
      dataContract,
    );

    if (!result.isValid()) {
      throw new InvalidDocumentError(result.getErrors(), rawDocument);
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
        ),
      );

      if (!result.isValid()) {
        throw new InvalidDocumentError(result.getErrors(), rawDocument);
      }
    }

    return dataContract;
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
    let rawDocument;
    try {
      // first 4 bytes are protocol version
      rawDocument = decode(buffer.slice(4, buffer.length));
      rawDocument.$protocolVersion = buffer.slice(0, 4).readUInt32BE(0);
    } catch (error) {
      throw new InvalidDocumentError([
        new SerializedObjectParsingError(
          buffer,
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
        if (!document.getOwnerId().equals(ownerId)) {
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

        const rawDocument = document.toObject();

        const keysToStay = [
          '$id',
          '$type',
          '$dataContractId',
          '$createdAt',
          '$updatedAt',
        ];

        Object.keys(rawDocument).forEach((key) => {
          if (key.startsWith('$') && !keysToStay.includes(key)) {
            delete rawDocument[key];
          }
        });

        return {
          ...rawDocument,
          $action: AbstractDocumentTransition.ACTIONS.CREATE,
          $entropy: document.getEntropy(),
        };
      });

    const rawDocumentReplaceTransitions = (replaceDocuments || [])
      .map((document) => {
        let rawDocument = document.toObject();

        const keysToStay = [
          '$id',
          '$type',
          '$dataContractId',
          '$revision',
          '$updatedAt',
        ];

        Object.keys(rawDocument).forEach((key) => {
          if (key.startsWith('$') && !keysToStay.includes(key)) {
            delete rawDocument[key];
          }
        });

        rawDocument = {
          ...rawDocument,
          $action: AbstractDocumentTransition.ACTIONS.REPLACE,
          $revision: rawDocument.$revision + 1,
        };

        // If document have an originally set `updatedAt`
        // we should update it then
        if (rawDocument.$updatedAt) {
          rawDocument.$updatedAt = new Date().getTime();
        }

        return rawDocument;
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
      protocolVersion: this.dpp.getProtocolVersion(),
      ownerId,
      transitions: rawDocumentTransitions,
    }, dataContracts);
  }
}

module.exports = DocumentFactory;

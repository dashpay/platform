const { promisify } = require('util');
const { join: pathJoin } = require('path');
const cbor = require('cbor');
const Document = require('@dashevo/dpp/lib/document/Document');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');

// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  driveOpen,
  driveClose,
  driveCreateRootTree,
  driveApplyContract,
  driveCreateDocument,
  driveUpdateDocument,
  driveDeleteDocument,
  driveQueryDocuments,
  driveInsertIdentity,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

const GroveDB = require('./GroveDB');

const appendStack = require('./appendStack');

const decodeProtocolEntity = decodeProtocolEntityFactory();

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = appendStack(promisify(driveClose));
const driveCreateRootTreeAsync = appendStack(promisify(driveCreateRootTree));
const driveApplyContractAsync = appendStack(promisify(driveApplyContract));
const driveCreateDocumentAsync = appendStack(promisify(driveCreateDocument));
const driveUpdateDocumentAsync = appendStack(promisify(driveUpdateDocument));
const driveDeleteDocumentAsync = appendStack(promisify(driveDeleteDocument));
const driveQueryDocumentsAsync = appendStack(promisify(driveQueryDocuments));
const driveInsertIdentityAsync = appendStack(promisify(driveInsertIdentity));

// Wrapper class for the boxed `Drive` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   */
  constructor(dbPath) {
    this.drive = driveOpen(dbPath);
    this.groveDB = new GroveDB(this.drive);
  }

  /**
   * @returns {GroveDB}
   */
  getGroveDB() {
    return this.groveDB;
  }

  /**
   * @returns {Promise<void>}
   */
  async close() {
    return driveCloseAsync.call(this.drive);
  }

  /**
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async createRootTree(useTransaction = false) {
    return driveCreateRootTreeAsync.call(this.drive, useTransaction);
  }

  /**
   * @param {DataContract} dataContract
   * @param {Date} blockTime
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async applyContract(dataContract, blockTime, useTransaction = false, dryRun = false) {
    return driveApplyContractAsync.call(
      this.drive,
      dataContract.toBuffer(),
      blockTime,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {Document} document
   * @param {Date} blockTime
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async createDocument(document, blockTime, useTransaction = false, dryRun = false) {
    return driveCreateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      true,
      blockTime,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {Document} document
   * @param {Date} blockTime
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async updateDocument(document, blockTime, useTransaction = false, dryRun = false) {
    return driveUpdateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      blockTime,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async deleteDocument(
    dataContract,
    documentType,
    documentId,
    useTransaction = false,
  ) {
    return driveDeleteDocumentAsync.call(
      this.drive,
      documentId.toBuffer(),
      dataContract.toBuffer(),
      documentType,
      useTransaction,
    );
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {Boolean} [useTransaction=false]
   *
   * @returns {Promise<[Document[], number]>}
   */
  async queryDocuments(dataContract, documentType, query = {}, useTransaction = false) {
    const encodedQuery = await cbor.encodeAsync(query);

    const [encodedDocuments, , processingFee] = await driveQueryDocumentsAsync.call(
      this.drive,
      encodedQuery,
      dataContract.id.toBuffer(),
      documentType,
      useTransaction,
    );

    const documents = encodedDocuments.map((encodedDocument) => {
      const [protocolVersion, rawDocument] = decodeProtocolEntity(encodedDocument);

      rawDocument.$protocolVersion = protocolVersion;

      return new Document(rawDocument, dataContract);
    });

    return [
      documents,
      processingFee,
    ];
  }

  /**
   * @param {Identity} identity
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async insertIdentity(identity, useTransaction = false, dryRun = false) {
    return driveInsertIdentityAsync.call(
      this.drive,
      identity.id.toBuffer(),
      identity.toBuffer(),
      !dryRun,
      useTransaction,
    );
  }
}

/**
 * @typedef Element
 * @property {string} type - element type. Can be "item", "reference" or "tree"
 * @property {Buffer|Buffer[]} value - element value
 */

module.exports = Drive;

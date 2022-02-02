const { promisify } = require('util');
const { join: pathJoin } = require('path');
const cbor = require('cbor');
const Document = require('@dashevo/dpp/lib/document/Document');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');

const decodeProtocolEntity = decodeProtocolEntityFactory();

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
  groveDbInsert,
  groveDbGet,
  groveDbFlush,
  groveDbStartTransaction,
  groveDbCommitTransaction,
  groveDbRollbackTransaction,
  groveDbIsTransactionStarted,
  groveDbAbortTransaction,
  groveDbDelete,
  groveDbInsertIfNotExists,
  groveDbPutAux,
  groveDbDeleteAux,
  groveDbGetAux,
  groveDbGetPathQuery,
  groveDbRootHash,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = promisify(driveClose);
const driveCreateRootTreeAsync = promisify(driveCreateRootTree);
const driveApplyContractAsync = promisify(driveApplyContract);
const driveCreateDocumentAsync = promisify(driveCreateDocument);
const driveUpdateDocumentAsync = promisify(driveUpdateDocument);
const driveDeleteDocumentAsync = promisify(driveDeleteDocument);
const driveQueryDocumentsAsync = promisify(driveQueryDocuments);

const groveDbGetAsync = promisify(groveDbGet);
const groveDbInsertAsync = promisify(groveDbInsert);
const groveDbInsertIfNotExistsAsync = promisify(groveDbInsertIfNotExists);
const groveDbDeleteAsync = promisify(groveDbDelete);
const groveDbFlushAsync = promisify(groveDbFlush);
const groveDbStartTransactionAsync = promisify(groveDbStartTransaction);
const groveDbCommitTransactionAsync = promisify(groveDbCommitTransaction);
const groveDbRollbackTransactionAsync = promisify(groveDbRollbackTransaction);
const groveDbIsTransactionStartedAsync = promisify(groveDbIsTransactionStarted);
const groveDbAbortTransactionAsync = promisify(groveDbAbortTransaction);
const groveDbPutAuxAsync = promisify(groveDbPutAux);
const groveDbDeleteAuxAsync = promisify(groveDbDeleteAux);
const groveDbGetAuxAsync = promisify(groveDbGetAux);
const groveDbGetPathQueryAsync = promisify(groveDbGetPathQuery);
const groveDbRootHashAsync = promisify(groveDbRootHash);

// Wrapper class for the boxed `GroveDB` for idiomatic JavaScript usage
class GroveDB {
  /**
   * @param drive
   */
  constructor(drive) {
    this.db = drive;
  }

  /**
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<Element>}
   */
  async get(path, key, useTransaction = false) {
    return groveDbGetAsync.call(this.db, path, key, useTransaction);
  }

  /**
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Element} value
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<*>}
   */
  async insert(path, key, value, useTransaction = false) {
    return groveDbInsertAsync.call(this.db, path, key, value, useTransaction);
  }

  /**
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Element} value
   * @param {boolean} [useTransaction=false]
   * @return {Promise<*>}
   */
  async insertIfNotExists(path, key, value, useTransaction = false) {
    return groveDbInsertIfNotExistsAsync.call(this.db, path, key, value, useTransaction);
  }

  /**
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {boolean} [useTransaction=false]
   * @return {Promise<*>}
   */
  async delete(path, key, useTransaction = false) {
    return groveDbDeleteAsync.call(this.db, path, key, useTransaction);
  }

  /**
   * Flush data on the disk
   *
   * @returns {Promise<void>}
   */
  async flush() {
    return groveDbFlushAsync.call(this.db);
  }

  /**
   * Start a transaction with isolated scope
   *
   * Write operations will be allowed only for the transaction
   * until it's committed
   *
   * @return {Promise<void>}
   */
  async startTransaction() {
    return groveDbStartTransactionAsync.call(this.db);
  }

  /**
   * Commit transaction
   *
   * Transaction should be started before
   *
   * @return {Promise<void>}
   */
  async commitTransaction() {
    return groveDbCommitTransactionAsync.call(this.db);
  }

  /**
   * Rollback transaction to this initial state when it was created
   *
   * @returns {Promise<void>}
   */
  async rollbackTransaction() {
    return groveDbRollbackTransactionAsync.call(this.db);
  }

  /**
   * Returns true if transaction started
   *
   * @returns {Promise<void>}
   */
  async isTransactionStarted() {
    return groveDbIsTransactionStartedAsync.call(this.db);
  }

  /**
   * Aborts transaction
   *
   * @returns {Promise<void>}
   */
  async abortTransaction() {
    return groveDbAbortTransactionAsync.call(this.db);
  }

  /**
   * Put auxiliary data
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {boolean} [useTransaction=false]
   * @return {Promise<*>}
   */
  async putAux(key, value, useTransaction = false) {
    return groveDbPutAuxAsync.call(this.db, key, value, useTransaction);
  }

  /**
   * Delete auxiliary data
   *
   * @param {Buffer} key
   * @param {boolean} [useTransaction=false]
   * @return {Promise<*>}
   */
  async deleteAux(key, useTransaction = false) {
    return groveDbDeleteAuxAsync.call(this.db, key, useTransaction);
  }

  /**
   * Get auxiliary data
   *
   * @param {Buffer} key
   * @param {boolean} [useTransaction=false]
   * @return {Promise<Buffer>}
   */
  async getAux(key, useTransaction = false) {
    return groveDbGetAuxAsync.call(this.db, key, useTransaction);
  }

  /**
   * Get data using query.
   *
   * @param {PathQuery} query
   * @param {boolean} [useTransaction=false]
   * @return {Promise<*>}
   */
  async getPathQuery(query, useTransaction = false) {
    return groveDbGetPathQueryAsync.call(this.db, query, useTransaction);
  }

  /**
   * Get root hash
   *
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async getRootHash(useTransaction = false) {
    return groveDbRootHashAsync.call(this.db, useTransaction);
  }
}

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
   * @returns {Promise<void>}
   */
  async createRootTree(useTransaction = false) {
    return driveCreateRootTreeAsync.call(this.drive, useTransaction);
  }

  /**
   * @param {DataContract} dataContract
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async applyContract(dataContract, useTransaction = false) {
    return driveApplyContractAsync.call(this.drive, dataContract.toBuffer(), useTransaction);
  }

  /**
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async createDocument(document, useTransaction = false) {
    return driveCreateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      true,
      useTransaction,
    );
  }

  /**
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async updateDocument(document, useTransaction = false) {
    return driveUpdateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContract().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      useTransaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
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
   * @returns {Promise<Document[]>}
   */
  async queryDocuments(dataContract, documentType, query = {}, useTransaction = false) {
    const encodedQuery = await cbor.encodeAsync(query);

    const [encodedDocuments] = await driveQueryDocumentsAsync.call(
      this.drive,
      encodedQuery,
      dataContract.toBuffer(),
      documentType,
      useTransaction,
    );

    return encodedDocuments.map((encodedDocument) => {
      const [protocolVersion, rawDocument] = decodeProtocolEntity(encodedDocument);

      rawDocument.$protocolVersion = protocolVersion;

      return new Document(rawDocument, dataContract);
    });
  }
}

/**
 * @typedef Element
 * @property {string} type - element type. Can be "item", "reference" or "tree"
 * @property {Buffer|Buffer[]} value - element value
 */

/**
 * @typedef PathQuery
 * @property {Buffer[]} path
 * @property {SizedQuery} query
 */

/**
 * @typedef SizedQuery
 * @property {Query} query
 * @property {Number|null} limit
 * @property {Number|null} offset
 */

/**
 * @typedef Query
 * @property {Array} items
 * @property {Buffer|null} subqueryKey
 * @property {Query|null} subquery
 * @property {boolean| null} leftToRight
 */

module.exports = Drive;

const { promisify } = require('util');
const { join: pathJoin } = require('path');

// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
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

module.exports = GroveDB;

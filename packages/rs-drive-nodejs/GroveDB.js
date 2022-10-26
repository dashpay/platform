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
  groveDbAbortTransaction,
  groveDbDelete,
  groveDbInsertIfNotExists,
  groveDbPutAux,
  groveDbDeleteAux,
  groveDbGetAux,
  groveDbQuery,
  groveDbProveQuery,
  groveDbRootHash,
  groveDbProveQueryMany,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

const appendStack = require('./appendStack');

const groveDbGetAsync = appendStack(promisify(groveDbGet));
const groveDbInsertAsync = appendStack(promisify(groveDbInsert));
const groveDbInsertIfNotExistsAsync = appendStack(promisify(groveDbInsertIfNotExists));
const groveDbDeleteAsync = appendStack(promisify(groveDbDelete));
const groveDbFlushAsync = appendStack(promisify(groveDbFlush));
const groveDbStartTransactionAsync = appendStack(promisify(groveDbStartTransaction));
const groveDbCommitTransactionAsync = appendStack(promisify(groveDbCommitTransaction));
const groveDbRollbackTransactionAsync = appendStack(promisify(groveDbRollbackTransaction));
const groveDbAbortTransactionAsync = appendStack(promisify(groveDbAbortTransaction));
const groveDbPutAuxAsync = appendStack(promisify(groveDbPutAux));
const groveDbDeleteAuxAsync = appendStack(promisify(groveDbDeleteAux));
const groveDbGetAuxAsync = appendStack(promisify(groveDbGetAux));
const groveDbQueryAsync = appendStack(promisify(groveDbQuery));
const groveDbProveQueryAsync = appendStack(promisify(groveDbProveQuery));
const groveDbProveQueryManyAsync = appendStack(promisify(groveDbProveQueryMany));
const groveDbRootHashAsync = appendStack(promisify(groveDbRootHash));

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
   * @param {GroveDBTransaction} [transaction=undefined]
   * @returns {Promise<Element>}
   */
  async get(path, key, transaction = undefined) {
    return groveDbGetAsync.call(this.db, path, key, transaction);
  }

  /**
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Element} value
   * @param {GroveDBTransaction} [transaction=undefined]
   * @returns {Promise<*>}
   */
  async insert(path, key, value, transaction = undefined) {
    return groveDbInsertAsync.call(this.db, path, key, value, transaction);
  }

  /**
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Element} value
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async insertIfNotExists(path, key, value, transaction = undefined) {
    return groveDbInsertIfNotExistsAsync.call(this.db, path, key, value, transaction);
  }

  /**
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async delete(path, key, transaction = undefined) {
    return groveDbDeleteAsync.call(this.db, path, key, transaction);
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
   * @return {Promise<GroveDBTransaction>}
   */
  async startTransaction() {
    return groveDbStartTransactionAsync.call(this.db);
  }

  /**
   * Commit transaction
   *
   * Transaction should be started before
   *
   * @param {GroveDBTransaction} [transaction=undefined]
   *
   * @return {Promise<void>}
   */
  async commitTransaction(transaction = undefined) {
    return groveDbCommitTransactionAsync.call(this.db, transaction);
  }

  /**
   * Rollback transaction to this initial state when it was created
   *
   * @param {GroveDBTransaction} [transaction=undefined]
   *
   * @returns {Promise<void>}
   */
  async rollbackTransaction(transaction = undefined) {
    return groveDbRollbackTransactionAsync.call(this.db, transaction);
  }

  /**
   * Aborts transaction
   *
   * @param {GroveDBTransaction} [transaction=undefined]
   *
   * @returns {Promise<void>}
   */
  async abortTransaction(transaction = undefined) {
    return groveDbAbortTransactionAsync.call(this.db, transaction);
  }

  /**
   * Put auxiliary data
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async putAux(key, value, transaction = undefined) {
    return groveDbPutAuxAsync.call(this.db, key, value, transaction);
  }

  /**
   * Delete auxiliary data
   *
   * @param {Buffer} key
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async deleteAux(key, transaction = undefined) {
    return groveDbDeleteAuxAsync.call(this.db, key, transaction);
  }

  /**
   * Get auxiliary data
   *
   * @param {Buffer} key
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<Buffer>}
   */
  async getAux(key, transaction = undefined) {
    return groveDbGetAuxAsync.call(this.db, key, transaction);
  }

  /**
   * Get data using query.
   *
   * @param {PathQuery} query
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async query(query, transaction = undefined) {
    return groveDbQueryAsync.call(this.db, query, transaction);
  }

  /**
   * Get proof using query.
   *
   * @param {PathQuery} query
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<*>}
   */
  async proveQuery(query, transaction = undefined) {
    return groveDbProveQueryAsync.call(this.db, query, transaction);
  }

  /**
   * Get proof using query.
   *
   * @param {PathQuery[]} queries
   * @param {GroveDBTransaction} [transaction=undefined]
   * @return {Promise<Buffer>}
   */
  async proveQueryMany(queries, transaction = undefined) {
    return groveDbProveQueryManyAsync.call(this.db, queries, transaction);
  }

  /**
   * Get root hash
   *
   * @param {GroveDBTransaction} [transaction=undefined]
   * @returns {Promise<void>}
   */
  async getRootHash(transaction = undefined) {
    return groveDbRootHashAsync.call(this.db, transaction);
  }
}

/**
 * @typedef GroveDBTransaction
 */

/**
 * @typedef Element
 * @property {"item"|"reference"|"tree"} type - element type. Can be "item", "reference" or "tree"
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
 * @property {number} [limit]
 * @property {number} [offset]
 */

/**
 * @typedef Query
 * @property {Array<
 *    QueryItemKey|
 *    QueryItemRange|
 *    QueryItemRangeInclusive|
 *    QueryItemRangeFull|
 *    QueryItemRangeFrom|
 *    QueryItemRangeTo|
 *    QueryItemRangeToInclusive|
 *    QueryItemRangeAfter|
 *    QueryItemRangeAfterTo|
 *    QueryItemRangeAfterToInclusive
 * >} [items]
 * @property {Buffer} [subqueryKey]
 * @property {Query} [subquery]
 * @property {boolean} [leftToRight]
 */

/**
 * @typedef QueryItemKey
 * @property {"key"} type
 * @property {Buffer} key
 */

/**
 * @typedef QueryItemRange
 * @property {"range"} type
 * @property {Buffer} from
 * @property {Buffer} to
 */

/**
 * @typedef QueryItemRangeInclusive
 * @property {"rangeInclusive"} type
 * @property {Buffer} from
 * @property {Buffer} to
 */

/**
 * @typedef QueryItemRangeFull
 * @property {"rangeFull"} type
 */

/**
 * @typedef QueryItemRangeFrom
 * @property {"rangeFrom"} type
 * @property {Buffer} from
 */

/**
 * @typedef QueryItemRangeTo
 * @property {"rangeTo"} type
 * @property {Buffer} to
 */

/**
 * @typedef QueryItemRangeToInclusive
 * @property {"rangeToInclusive"} type
 * @property {Buffer} to
 */

/**
 * @typedef QueryItemRangeAfter
 * @property {"rangeAfter"} type
 * @property {Buffer} after
 */

/**
 * @typedef QueryItemRangeAfterTo
 * @property {"rangeAfterTo"} type
 * @property {Buffer} after
 * @property {Buffer} to
 */

/**
 * @typedef QueryItemRangeAfterToInclusive
 * @property {"rangeAfterToInclusive"} type
 * @property {Buffer} after
 * @property {Buffer} to
 */

module.exports = GroveDB;

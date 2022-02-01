const { promisify } = require('util');
const { join: pathJoin } = require('path');
// const GroveDB = require('@dashevo/grovedb');

// GroveDB.prototype.constructor = function () {
//
// }

// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  driveOpen,
  driveClose,
  driveCreateRootTree,
  // driveGetGroveDB,
  groveDbInsert,
  groveDbClose,
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

// Convert the DB methods from using callbacks to returning promises
const driveCloseAsync = promisify(driveClose);
const driveCreateRootTreeAsync = promisify(driveCreateRootTree);

// Wrapper class for the boxed `Database` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   */
  constructor(dbPath) {
    this.drive = driveOpen(dbPath);
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
}

module.exports = Drive;

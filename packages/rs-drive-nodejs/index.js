const { promisify } = require('util');
const { join: pathJoin } = require('path');
// const GroveDB = require('@dashevo/grovedb');

// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  driveOpen,
  driveClose,
  driveCreateRootTree,
  driveApplyContract,
  driveCreateDocument,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

// function GroveDBFromDrive(groveDB) {
//   this.db = groveDB;
// }
//
// GroveDBFromDrive.prototype = GroveDB.prototype;

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = promisify(driveClose);
const driveCreateRootTreeAsync = promisify(driveCreateRootTree);
const driveApplyContractAsync = promisify(driveApplyContract);
const driveCreateDocumentAsync = promisify(driveCreateDocument);

// Wrapper class for the boxed `Drive` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   */
  constructor(dbPath) {
    this.drive = driveOpen(dbPath);
  }

  // /**
  //  * @returns {GroveDB|GroveDBFromDrive}
  //  */
  // getGroveDB() {
  //   const groveDBWrapper = driveGetGroveDB.call(this.drive);
  //
  //   return new GroveDBFromDrive(groveDBWrapper);
  // }

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
   * @param {Buffer} encodedContract
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async applyContract(encodedContract, useTransaction = false) {
    return driveApplyContractAsync.call(this.drive, encodedContract, useTransaction);
  }

  /**
   * @param {Buffer} encodedDocument
   * @param {Buffer} encodedContract
   * @param {string} documentType
   * @param {Buffer} ownerId
   * @param {boolean} [isOverride=false]
   * @param {boolean} [useTransaction=false]
   * @returns {Promise<void>}
   */
  async createDocument(
    encodedDocument,
    encodedContract,
    documentType,
    ownerId,
    isOverride = false,
    useTransaction = false,
  ) {
    return driveCreateDocumentAsync.call(
      this.drive,
      encodedDocument,
      encodedContract,
      documentType,
      ownerId,
      isOverride,
      useTransaction,
    );
  }
}

module.exports = Drive;

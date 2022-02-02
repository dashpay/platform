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
  driveUpdateDocument,
  driveDeleteDocument,
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
const driveUpdateDocumentAsync = promisify(driveUpdateDocument);
const driveDeleteDocumentAsync = promisify(driveDeleteDocument);

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
   * @param contract
   * @param type
   * @param query
   * @returns {Promise<void>}
   */
  async findDocuments(contract, type, query) {

  }
}

module.exports = Drive;

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
  driveCreateInitialStateStructure,
  driveApplyContract,
  driveCreateDocument,
  driveUpdateDocument,
  driveDeleteDocument,
  driveQueryDocuments,
  driveProveDocumentsQuery,
  driveInsertIdentity,
  driveFetchLatestWithdrawalTransactionIndex,
  driveEnqueueWithdrawalTransaction,
  abciInitChain,
  abciBlockBegin,
  abciBlockEnd,
} = require('neon-load-or-build')({
  dir: pathJoin(__dirname, '..'),
});

const GroveDB = require('./GroveDB');

const appendStack = require('./appendStack');

const decodeProtocolEntity = decodeProtocolEntityFactory();

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = appendStack(promisify(driveClose));
const driveCreateInitialStateStructureAsync = appendStack(
  promisify(driveCreateInitialStateStructure),
);
const driveApplyContractAsync = appendStack(promisify(driveApplyContract));
const driveCreateDocumentAsync = appendStack(promisify(driveCreateDocument));
const driveUpdateDocumentAsync = appendStack(promisify(driveUpdateDocument));
const driveDeleteDocumentAsync = appendStack(promisify(driveDeleteDocument));
const driveQueryDocumentsAsync = appendStack(promisify(driveQueryDocuments));
const driveProveDocumentsQueryAsync = appendStack(promisify(driveProveDocumentsQuery));
const driveFetchLatestWithdrawalTransactionIndexAsync = appendStack(
  promisify(driveFetchLatestWithdrawalTransactionIndex),
);
const driveEnqueueWithdrawalTransactionAsync = appendStack(
  promisify(driveEnqueueWithdrawalTransaction),
);
const driveInsertIdentityAsync = appendStack(promisify(driveInsertIdentity));
const abciInitChainAsync = appendStack(promisify(abciInitChain));
const abciBlockBeginAsync = appendStack(promisify(abciBlockBegin));
const abciBlockEndAsync = appendStack(promisify(abciBlockEnd));

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
  async createInitialStateStructure(useTransaction = false) {
    return driveCreateInitialStateStructureAsync.call(this.drive, useTransaction);
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
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<[number, number]>}
   */
  async deleteDocument(
    dataContract,
    documentType,
    documentId,
    useTransaction = false,
    dryRun = false,
  ) {
    return driveDeleteDocumentAsync.call(
      this.drive,
      documentId.toBuffer(),
      dataContract.toBuffer(),
      documentType,
      !dryRun,
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
  async proveDocumentsQuery(dataContract, documentType, query = {}, useTransaction = false) {
    const encodedQuery = await cbor.encodeAsync(query);

    // eslint-disable-next-line no-return-await
    return await driveProveDocumentsQueryAsync.call(
      this.drive,
      encodedQuery,
      dataContract.id.toBuffer(),
      documentType,
      useTransaction,
    );
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
      identity.toBuffer(),
      !dryRun,
      useTransaction,
    );
  }

  /**
   * Fetch latest index of the withdrawal transaction in a queue
   *
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<number>}
   */
  async fetchLatestWithdrawalTransactionIndex(useTransaction = false) {
    return driveFetchLatestWithdrawalTransactionIndexAsync.call(
      this.drive,
      useTransaction,
    );
  }

  /**
   * Enqueue withdrawal transaction into the queue
   *
   * @param {number} index
   * @param {Buffer} transactionBytes
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<void>}
   */
  async enqueueWithdrawalTransaction(index, transactionBytes, useTransaction = false) {
    return driveEnqueueWithdrawalTransactionAsync.call(
      this.drive,
      index,
      transactionBytes,
      useTransaction,
    );
  }

  /**
   * Get the ABCI interface
   * @returns {RSAbci}
   */
  getAbci() {
    const { drive } = this;

    /**
     * @typedef RSAbci
     */
    return {
      /**
       * ABCI init chain
       *
       * @param {InitChainRequest} request
       * @param {boolean} [useTransaction=false]
       *
       * @returns {Promise<InitChainResponse>}
       */
      async initChain(request, useTransaction = false) {
        const requestBytes = cbor.encode(request);

        const responseBytes = await abciInitChainAsync.call(
          drive,
          requestBytes,
          useTransaction,
        );

        return cbor.decode(responseBytes);
      },

      /**
       * ABCI init chain
       *
       * @param {BlockBeginRequest} request
       * @param {boolean} [useTransaction=false]
       *
       * @returns {Promise<BlockBeginResponse>}
       */
      async blockBegin(request, useTransaction = false) {
        const requestBytes = cbor.encode({
          ...request,
          // cborium doesn't eat Buffers
          proposerProTxHash: Array.from(request.proposerProTxHash),
          validatorSetQuorumHash: Array.from(request.validatorSetQuorumHash),
        });

        const responseBytes = await abciBlockBeginAsync.call(
          drive,
          requestBytes,
          useTransaction,
        );

        return cbor.decode(responseBytes);
      },

      /**
       * ABCI init chain
       *
       * @param {BlockEndRequest} request
       * @param {boolean} [useTransaction=false]
       *
       * @returns {Promise<BlockEndResponse>}
       */
      async blockEnd(request, useTransaction = false) {
        const requestBytes = cbor.encode(request);

        const responseBytes = await abciBlockEndAsync.call(
          drive,
          requestBytes,
          useTransaction,
        );

        return cbor.decode(responseBytes);
      },
    };
  }
}

/**
 * @typedef InitChainRequest
 */

/**
 * @typedef InitChainResponse
 */

/**
 * @typedef BlockBeginRequest
 * @property {number} blockHeight
 * @property {number} blockTimeMs - timestamp in milliseconds
 * @property {number} [previousBlockTimeMs] - timestamp in milliseconds
 * @property {Buffer} proposerProTxHash
 * @property {Buffer} validatorSetQuorumHash
 */

/**
 * @typedef BlockBeginResponse
 * @property {Buffer[]} unsignedWithdrawalTransactions
 */

/**
 * @typedef BlockEndRequest
 * @property {Fees} fees
 */

/**
 * @typedef Fees
 * @property {number} processingFees
 * @property {number} storageFees
 */

/**
 * @typedef BlockEndResponse
 * @property {number} currentEpochIndex
 * @property {boolean} isEpochChange
 * @property {number} [proposersPaidCount]
 * @property {number} [paidEpochIndex]
 */

module.exports = Drive;

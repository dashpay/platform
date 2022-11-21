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
  driveFetchContract,
  driveCreateContract,
  driveUpdateContract,
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
  abciAfterFinalizeBlock,
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
const driveFetchContractAsync = appendStack(promisify(driveFetchContract));
const driveCreateContractAsync = appendStack(promisify(driveCreateContract));
const driveUpdateContractAsync = appendStack(promisify(driveUpdateContract));
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
const abciAfterFinalizeBlockAsync = appendStack(promisify(abciAfterFinalizeBlock));

// Wrapper class for the boxed `Drive` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   * @param {Object} config
   * @param {number} config.dataContractsGlobalCacheSize
   * @param {number} config.dataContractsTransactionalCacheSize
   */
  constructor(dbPath, config) {
    this.drive = driveOpen(dbPath, config);
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
   * @param {Buffer|Identifier} id
   * @param {number} epochIndex
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<[DataContract, FeeResult]>}
   */
  async fetchContract(id, epochIndex = undefined, useTransaction = false) {
    return driveFetchContractAsync.call(
      this.drive,
      Buffer.from(id),
      epochIndex,
      useTransaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async createContract(dataContract, blockInfo, useTransaction = false, dryRun = false) {
    return driveCreateContractAsync.call(
      this.drive,
      dataContract.toBuffer(),
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async updateContract(dataContract, blockInfo, useTransaction = false, dryRun = false) {
    return driveUpdateContractAsync.call(
      this.drive,
      dataContract.toBuffer(),
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {Document} document
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async createDocument(document, blockInfo, useTransaction = false, dryRun = false) {
    return driveCreateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContractId().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      true,
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {Document} document
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async updateDocument(document, blockInfo, useTransaction = false, dryRun = false) {
    return driveUpdateDocumentAsync.call(
      this.drive,
      document.toBuffer(),
      document.getDataContractId().toBuffer(),
      document.getType(),
      document.getOwnerId().toBuffer(),
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * @param {Buffer|Identifier} dataContractId
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async deleteDocument(
    dataContractId,
    documentType,
    documentId,
    blockInfo,
    useTransaction = false,
    dryRun = false,
  ) {
    return driveDeleteDocumentAsync.call(
      this.drive,
      documentId.toBuffer(),
      dataContractId,
      documentType,
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {number} [epochIndex]
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
  async queryDocuments(
    dataContract,
    documentType,
    epochIndex = undefined,
    query = {},
    useTransaction = false,
  ) {
    const encodedQuery = await cbor.encodeAsync(query);

    const [encodedDocuments, , processingFee] = await driveQueryDocumentsAsync.call(
      this.drive,
      encodedQuery,
      dataContract.id.toBuffer(),
      documentType,
      epochIndex,
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
   * @param {BlockInfo} blockInfo
   * @param {boolean} [useTransaction=false]
   * @param {boolean} [dryRun=false]
   *
   * @returns {Promise<FeeResult>}
   */
  async insertIdentity(identity, blockInfo, useTransaction = false, dryRun = false) {
    return driveInsertIdentityAsync.call(
      this.drive,
      identity.toBuffer(),
      blockInfo,
      !dryRun,
      useTransaction,
    );
  }

  /**
   * Fetch the latest index of the withdrawal transaction in a queue
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
       * ABCI block begin
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
       * ABCI block end
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

      /**
       * ABCI after finalize block
       *
       * @param {AfterFinalizeBlockRequest} request
       *
       * @returns {Promise<AfterFinalizeBlockResponse>}
       */
      async afterFinalizeBlock(request) {
        const requestBytes = cbor.encode({
          ...request,
          // cborium doesn't eat Buffers
          updatedDataContractIds: request.updatedDataContractIds
            .map((identifier) => Array.from(identifier)),
        });

        const responseBytes = await abciAfterFinalizeBlockAsync.call(
          drive,
          requestBytes,
        );

        return cbor.decode(responseBytes);
      },
    };
  }
}

/**
 * @typedef BlockInfo
 * @property {number} height
 * @property {number} epoch
 * @property {number} timeMs
 */

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
 * @property {EpochInfo} epochInfo
 */

/**
 * @typedef EpochInfo
 * @property {number} currentEpochIndex
 * @property {boolean} isEpochChange
 * @property {number} [previousEpochIndex] - Available only on epoch change
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
 * @property {number} [proposersPaidCount]
 * @property {number} [paidEpochIndex]
 */

/**
 * @typedef AfterFinalizeBlockRequest
 * @property {Identifier[]|Buffer[]} updatedDataContractIds
 */

/**
 * @typedef AfterFinalizeBlockResponse
 */

module.exports = Drive;

const { promisify } = require('util');
const cbor = require('cbor');
const Document = require('@dashevo/dpp/lib/document/Document');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
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
  calculateStorageFeeDistributionAmountAndLeftovers,
} = require('neon-load-or-build')({
  dir: __dirname,
});

const GroveDB = require('./GroveDB');
const FeeResult = require('./FeeResult');

const { appendStackAsync, appendStack } = require('./appendStack');

const decodeProtocolEntity = decodeProtocolEntityFactory();

// Convert the Drive methods from using callbacks to returning promises
const driveCloseAsync = appendStackAsync(promisify(driveClose));
const driveCreateInitialStateStructureAsync = appendStackAsync(
  promisify(driveCreateInitialStateStructure),
);
const driveFetchContractAsync = appendStackAsync(promisify(driveFetchContract));
const driveCreateContractAsync = appendStackAsync(promisify(driveCreateContract));
const driveUpdateContractAsync = appendStackAsync(promisify(driveUpdateContract));
const driveCreateDocumentAsync = appendStackAsync(promisify(driveCreateDocument));
const driveUpdateDocumentAsync = appendStackAsync(promisify(driveUpdateDocument));
const driveDeleteDocumentAsync = appendStackAsync(promisify(driveDeleteDocument));
const driveQueryDocumentsAsync = appendStackAsync(promisify(driveQueryDocuments));
const driveProveDocumentsQueryAsync = appendStackAsync(promisify(driveProveDocumentsQuery));
const driveFetchLatestWithdrawalTransactionIndexAsync = appendStackAsync(
  promisify(driveFetchLatestWithdrawalTransactionIndex),
);
const driveEnqueueWithdrawalTransactionAsync = appendStackAsync(
  promisify(driveEnqueueWithdrawalTransaction),
);
const driveInsertIdentityAsync = appendStackAsync(promisify(driveInsertIdentity));
const abciInitChainAsync = appendStackAsync(promisify(abciInitChain));
const abciBlockBeginAsync = appendStackAsync(promisify(abciBlockBegin));
const abciBlockEndAsync = appendStackAsync(promisify(abciBlockEnd));
const abciAfterFinalizeBlockAsync = appendStackAsync(promisify(abciAfterFinalizeBlock));

const calculateStorageFeeDistributionAmountAndLeftoversWithStack = appendStack(
  calculateStorageFeeDistributionAmountAndLeftovers,
);

// Wrapper class for the boxed `Drive` for idiomatic JavaScript usage
class Drive {
  /**
   * @param {string} dbPath
   * @param {Object} config
   * @param {number} config.dataContractsGlobalCacheSize
   * @param {number} config.dataContractsBlockCacheSize
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
   * @param {number} [epochIndex]
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<[DataContract|null, FeeResult]>}
   */
  async fetchContract(id, epochIndex = undefined, useTransaction = false) {
    return driveFetchContractAsync.call(
      this.drive,
      Buffer.from(id),
      epochIndex,
      useTransaction,
    ).then(([encodedDataContract, innerFeeResult]) => {
      let dataContract = encodedDataContract;

      if (encodedDataContract !== null) {
        const [protocolVersion, rawDataContract] = decodeProtocolEntity(
          encodedDataContract,
        );

        rawDataContract.protocolVersion = protocolVersion;

        dataContract = new DataContract(rawDataContract);
      }

      const result = [dataContract];

      if (innerFeeResult) {
        result.push(new FeeResult(innerFeeResult));
      }

      return result;
    });
  }

  /**
   * @param {DataContract} dataContract
   * @param {RawBlockInfo} blockInfo
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
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
  }

  /**
   * @param {DataContract} dataContract
   * @param {RawBlockInfo} blockInfo
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
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
  }

  /**
   * @param {Document} document
   * @param {RawBlockInfo} blockInfo
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
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
  }

  /**
   * @param {Document} document
   * @param {RawBlockInfo} blockInfo
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
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
  }

  /**
   * @param {Identifier} dataContractId
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {RawBlockInfo} blockInfo
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
      dataContractId.toBuffer(),
      documentType,
      blockInfo,
      !dryRun,
      useTransaction,
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
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
   * @param {boolean} [useTransaction=false]
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
      dataContract.getId().toBuffer(),
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
   * @param {boolean} [useTransaction=false]
   *
   * @returns {Promise<[Document[], number]>}
   */
  async proveDocumentsQuery(dataContract, documentType, query = {}, useTransaction = false) {
    const encodedQuery = await cbor.encodeAsync(query);

    // eslint-disable-next-line no-return-await
    return await driveProveDocumentsQueryAsync.call(
      this.drive,
      encodedQuery,
      dataContract.getId().toBuffer(),
      documentType,
      useTransaction,
    );
  }

  /**
   * @param {Identity} identity
   * @param {RawBlockInfo} blockInfo
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
    ).then((innerFeeResult) => new FeeResult(innerFeeResult));
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
        const responseBytes = await abciBlockEndAsync.call(
          drive,
          request,
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

// eslint-disable-next-line max-len
Drive.calculateStorageFeeDistributionAmountAndLeftovers = calculateStorageFeeDistributionAmountAndLeftoversWithStack;

/**
 * @typedef RawBlockInfo
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
 * @property {BlockFeeResult} fees
 */

/**
 * @typedef BlockFeeResult
 * @property {number} storageFee
 * @property {number} processingFee
 * @property {Object<string, number>} feeRefunds
 * @property {number} feeRefundsSum
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

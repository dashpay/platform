const { EventEmitter } = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const { createBloomFilter, parseRawTransactions, parseRawMerkleBlock } = require('./utils');

const EVENTS = {
  HISTORICAL_TRANSACTIONS: 'TRANSACTIONS',
  MERKLE_BLOCK: 'MERKLE_BLOCK',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

/**
 * @typedef TransactionsReaderOptions
 * @property {Function} [createHistoricalSyncStream]
 * @property {Function} [createContinuousSyncStream]
 * @property {string} network
 */

class TransactionsReader extends EventEmitter {
  /**
   * @param {TransactionsReaderOptions} options
   */
  constructor(options = {}) {
    super();
    this.createHistoricalSyncStream = options.createHistoricalSyncStream;
    this.createContinuousSyncStream = options.createContinuousSyncStream;
    this.network = options.network;

    this.historicalSyncStream = null;
    this.createContinuousSyncStream = null;
  }

  /**
   * Reads historical transactions
   *
   * @param {number} fromBlockHeight
   * @param {number} toBlockHeight
   * @param {string[]} addresses
   * @returns {Promise<void>}
   */
  async startHistoricalSync(fromBlockHeight, toBlockHeight, addresses) {
    if (this.historicalSyncStream) {
      throw new Error('Historical sync is already in process.');
    }

    if (fromBlockHeight <= 1) {
      throw new Error('Invalid fromBlockHeight');
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount < 0) {
      throw new Error(`Invalid total amount of blocks to sync: ${totalAmount}`);
    }

    // this.historicalSyncStream = await
  }

  /**
   * @private
   *
   * @param {number} fromBlockHeight
   * @param {number} count
   * @param {string[]} addresses
   * @return Promise<!grpc.web.ClientReadableStream>
   */
  async subscribeToHistoricalStream(fromBlockHeight, count, addresses) {
    const bloomFilter = createBloomFilter(addresses);
    const stream = await this.createHistoricalSyncStream(fromBlockHeight, count, bloomFilter);

    // Arguments for the stream restart when it comes to a need to expand bloom filter
    let restartArgs = null;

    const dataHandler = (data) => {
      const rawTransactions = data.getRawTransactions();
      // TBD: const rawInstantLocks = data.getInstantSendLockMessages();
      const rawMerkleBlock = data.getRawMerkleBlock();

      if (rawTransactions) {
        const transactions = parseRawTransactions(rawTransactions, addresses, this.network);

        if (transactions.length) {
          this.emit(EVENTS.HISTORICAL_TRANSACTIONS, transactions);
        }
      } else if (rawMerkleBlock) {
        const merkleBlock = parseRawMerkleBlock(rawMerkleBlock);

        let rejected = false;
        let accepted = false;

        /**
         * Accepts merkle block
         * @param {number} height
         * @param {string[]} newAddresses
         */
        const acceptMerkleBlock = (height, newAddresses) => {
          if (rejected) {
            throw new Error('Unable to accept rejected merkle block');
          }
          accepted = true;

          if (newAddresses.length) {
            const blocksRead = height - fromBlockHeight + 1;
            const remainingCount = count - blocksRead;
            if (remainingCount === 0) {
              return;
            }

            if (remainingCount < 0) {
              const error = new Error(`Merkle block height is greater than expected range: ${height} > ${fromBlockHeight + count - 1}`);
              stream.destroy(error);
              return;
            }

            restartArgs = {
              fromBlockHeight: height + 1,
              count: remainingCount,
              addresses: [...addresses, ...newAddresses],
            };

            // Restart stream to expand bloom filter
            stream.cancel()
              .catch((e) => this.emit(EVENTS.ERROR, e));
          }
        };

        const rejectMerkleBlock = (e) => {
          if (accepted) {
            throw new Error('Unable to reject accepted merkle block');
          }
          rejected = true;
          stream.destroy(e);
        };

        this.emit(EVENTS.MERKLE_BLOCK, { merkleBlock, acceptMerkleBlock, rejectMerkleBlock });
      }
    };

    const errorHandler = (streamError) => {
      if (streamError.code === GrpcErrorCodes.CANCELLED) {
        this.historicalSyncStream = null;
        if (restartArgs) {
          this.subscribeToHistoricalStream(
            restartArgs.fromBlockHeight,
            restartArgs.count,
            restartArgs.addresses,
          ).then((newStream) => {
            this.historicalSyncStream = newStream;
          }).catch((e) => {
            this.emit(EVENTS.ERROR, e);
          });

          restartArgs = null;
        }

        return;
      }

      this.emit(EVENTS.ERROR, streamError);
    };

    const endHandler = () => {
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
    };

    stream.on('data', dataHandler);
    stream.on('error', errorHandler);
    stream.on('end', endHandler);

    return stream;
  }

  async stopReadingHistorical() {
    if (this.historicalSyncStream) {
      await this.historicalSyncStream.cancel();
    }
  }
}

TransactionsReader.EVENTS = EVENTS;

module.exports = TransactionsReader;

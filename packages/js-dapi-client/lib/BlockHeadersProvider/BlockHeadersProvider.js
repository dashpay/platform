const EventEmitter = require('events');
const { SpvChain } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

const EVENTS = {
  HISTORICAL_SYNC_FINISHED: 'HISTORICAL_SYNC_FINISHED',
  BATCH_OF_HEADERS_VERIFIED: 'BATCH_OF_HEADERS_VERIFIED',
  ERROR: 'ERROR',
};

/**
 * @typedef {BlockHeadersProviderOptions} BlockHeadersProviderOptions
 * @property {string} [network=testnet]
 * @property {number} [maxParallelStreams=5] max parallel streams to read historical block headers
 * @property {number} [targetBatchSize=100000] a target batch size per stream
 * @property {number} [maxRetries=10] max amount of retries per stream connection
 * @property {number} [autoStart=false] auto start fetching verifying block headers
 */
const defaultOptions = {
  network: 'testnet',
  maxParallelStreams: 5,
  targetBatchSize: 100000,
  fromBlockHeight: 1,
  maxRetries: 10,
  autoStart: false,
};

class BlockHeadersProvider extends EventEmitter {
  /**
   * @param {BlockHeadersProviderOptions} options
   */
  constructor(options = {}) {
    super();

    this.options = {
      ...defaultOptions,
      ...options,
    };

    // TODO: test - remove second argument
    this.spvChain = new SpvChain(this.options.network, 100000);
    this.started = false;
  }

  /**
   * @param {CoreMethodsFacade} coreMethods
   */
  setCoreMethods(coreMethods) {
    this.coreMethods = coreMethods;
  }

  async start() {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    if (this.started) {
      throw new Error('BlockHeadersProvider has already been started');
    }

    this.started = true;

    try {
      const { chain: { blocksCount: bestBlockHeight } } = await this.coreMethods.getStatus();
      const blockHeadersReader = new BlockHeadersReader(
        {
          coreMethods: this.coreMethods,
          maxParallelStreams: this.options.maxParallelStreams,
          targetBatchSize: this.options.targetBatchSize,
          maxRetries: this.options.maxRetries,
        },
      );

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        throw e;
      });

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers, reject) => {
        try {
          this.spvChain.addHeaders(headers.map((header) => Buffer.from(header)));
          this.emit(EVENTS.BATCH_OF_HEADERS_VERIFIED, headers);
        } catch (e) {
          if (e.message === 'Some headers are invalid') {
            reject(e);
          } else {
            throw e;
          }
        }
      });

      await blockHeadersReader.readHistorical(
        this.options.fromBlockHeight,
        bestBlockHeight - 1,
      );

      this.emit(EVENTS.HISTORICAL_SYNC_FINISHED);

      await blockHeadersReader.subscribeToNew(bestBlockHeight);
    } catch (e) {
      this.started = false;
      throw e;
    }
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.defaultOptions = defaultOptions;

module.exports = BlockHeadersProvider;

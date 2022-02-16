const EventEmitter = require('events');
const { SpvChain } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

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

const EVENTS = {
  ERROR: 'error',
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

    this.spvChain = new SpvChain(this.options.network);
    this.started = false;
  }

  /**
   * @param {CoreMethodsFacade} coreMethods
   */
  setCoreMethods(coreMethods) {
    this.coreMethods = coreMethods;
  }

  /**
   * @param {BlockHeadersReader} blockHeadersReader
   */
  setBlockHeadersReader(blockHeadersReader) {
    this.blockHeadersReader = blockHeadersReader;
  }

  /**
   *
   * @param spvChain
   */
  setSpvChain(spvChain) {
    this.spvChain = spvChain;
  }

  async start() {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    if (this.started) {
      throw new Error('BlockHeaderProvider has already been started');
    }

    const { chain: { blocksCount: bestBlockHeight } } = await this.coreMethods.getStatus();

    if (!this.blockHeadersReader) {
      this.blockHeadersReader = new BlockHeadersReader(
        {
          coreMethods: this.coreMethods,
          maxParallelStreams: this.options.maxParallelStreams,
          targetBatchSize: this.options.targetBatchSize,
          maxRetries: this.options.maxRetries,
        },
      );
    }

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
      this.emit(EVENTS.ERROR, e);
    });

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
      this.blockHeadersReader.subscribeToNew(bestBlockHeight)
        .catch((e) => {
          this.emit(EVENTS.ERROR, e);
        });
    });

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers, reject) => {
      try {
        this.spvChain.addHeaders(headers.map((header) => Buffer.from(header)));
      } catch (e) {
        if (e.message === 'Some headers are invalid') {
          reject(e);
        } else {
          this.emit(EVENTS.ERROR, e);
        }
      }
    });

    await this.blockHeadersReader.readHistorical(
      this.options.fromBlockHeight,
      bestBlockHeight - 1,
    );

    this.started = true;
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.defaultOptions = defaultOptions;

module.exports = BlockHeadersProvider;

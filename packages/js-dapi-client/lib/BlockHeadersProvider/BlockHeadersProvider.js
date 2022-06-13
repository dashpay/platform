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
  maxParallelStreams: 10,
  targetBatchSize: 50000,
  fromBlockHeight: 1,
  maxRetries: 10,
  autoStart: false,
};

const EVENTS = {
  ERROR: 'error',
  CHAIN_UPDATED: 'CHAIN_UPDATED',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
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

    // TODO: make sure it's okay passing 0 parameter here
    this.spvChain = new SpvChain(this.options.network, 1);
    this.started = false;
    // TODO: write tests for this
    this.headersHeights = {};
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

  /**
   * Inits block headers stream
   *
   * @param fromBlockHeight
   * @param toBlockHeight
   * @returns {Promise<void>}
   */
  // 2cbcf83b62913d56f605c0e581a48872839428c92e5eb76cd7ad94bcaf0b0000
  // 2cbcf83b62913d56f605c0e581a48872839428c92e5eb76cd7ad94bcaf0b0000
  async start(fromBlockHeight = 1, toBlockHeight) {
    if (!this.coreMethods) {
      throw new Error('Core methods have not been provided. Please use "setCoreMethods"');
    }

    if (this.started) {
      throw new Error('BlockHeaderProvider has already been started');
    }

    // const { chain: { blocksCount: bestBlockHeight } } = await this.coreMethods.getStatus();

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
    const batches = [];
    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      this.blockHeadersReader.subscribeToNew(toBlockHeight)
        .catch((e) => {
          this.emit(EVENTS.ERROR, e);
        });
    });

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS,
      (headers, headHeight, reject) => {
        try {
        // const headersBuffers = rawHeaders.map((header) => Buffer.from(header));
          batches.push(headers.map((header) => header.toString('hex')));
          // const headers = rawHeaders.map((header) => new BlockHeader(Buffer.from(header)));
          this.spvChain.addHeaders(headers);

          headers.forEach((header, index) => {
            this.headersHeights[header.hash] = headHeight + index;
          });
          console.log('Obtained', headers.length, Object.keys(this.headersHeights).length);

          // console.log('Total orphans', totalOrphans);
          this.emit(EVENTS.CHAIN_UPDATED, this.spvChain.allHeaders, this.spvChain.orphanChunks);
        } catch (e) {
          if (e.message === 'Some headers are invalid') {
            reject(e);
          } else {
            this.emit(EVENTS.ERROR, e);
          }
        }
      });

    await this.blockHeadersReader.readHistorical(
      fromBlockHeight,
      toBlockHeight,
    );

    this.started = true;
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.defaultOptions = defaultOptions;

module.exports = BlockHeadersProvider;

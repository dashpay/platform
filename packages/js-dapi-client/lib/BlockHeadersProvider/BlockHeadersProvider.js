const EventEmitter = require('events');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');

const BlockHeadersReader = require('./BlockHeadersReader');

/**
 * @typedef {BlockHeadersProviderOptions} BlockHeadersProviderOptions
 * @property {string} [network='testnet'] network type
 * @property {number} [maxParallelStreams=10] max parallel streams to read historical block headers
 * @property {number} [targetBatchSize=10000] a target batch size per stream
 * @property {number} [fromBlockHeight=1] a target batch size per stream
 * @property {number} [maxRetries=10] max amount of retries per stream connection
 * @property {number} [powTargetSpacing] devnets only: the -powtargetspacing the
 *   devnet was started with
 * @property {number} [minimumDifficultyBlocks] devnets only: the
 *   -minimumdifficultyblocks the devnet was started with
 */
const defaultOptions = {
  network: 'testnet',
  maxParallelStreams: 10,
  // Headers read ahead of the chain wait in memory, up to about
  // maxParallelStreams * targetBatchSize of them
  targetBatchSize: 10000,
  fromBlockHeight: 1,
  maxRetries: 10,
};

const EVENTS = {
  ERROR: 'error',
  HISTORICAL_SYNC_STARTED: 'HISTORICAL_SYNC_STARTED',
  CONTINUOUS_SYNC_STARTED: 'CONTINUOUS_SYNC_STARTED',
  CHAIN_UPDATED: 'CHAIN_UPDATED',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  STOPPED: 'STOPPED',
};

const STATES = {
  IDLE: 'IDLE',
  HISTORICAL_SYNC: 'HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'CONTINUOUS_SYNC',
};

class BlockHeadersProvider extends EventEmitter {
  /**
   * @param {BlockHeadersProviderOptions} options for block headers
   * @param {Function} [createHistoricalSyncStream] createHistoricalSyncStream
   * @param {Function} [createContinuousSyncStream] createContinuousSyncStream
   */
  // TODO move options to as last param
  // eslint-disable-next-line default-param-last
  constructor(options = {}, createHistoricalSyncStream, createContinuousSyncStream) {
    super();
    this.options = {
      ...defaultOptions,
      ...options,
    };

    this.state = STATES.IDLE;

    // How far below the tip continuous sync asks for headers again when new
    // ones don't connect: DAPI streams only each new tip, so after a reorg or
    // several blocks at once the parents have to be fetched by height
    this.continuousLookback = 1;

    this.errorHandler = this.errorHandler.bind(this);
    this.headersHandler = this.headersHandler.bind(this);
    this.chainConnectedHandler = this.chainConnectedHandler.bind(this);
    this.historicalDataObtainedHandler = this.historicalDataObtainedHandler.bind(this);
    this.createHistoricalSyncStream = createHistoricalSyncStream;
    this.createContinuousSyncStream = createContinuousSyncStream;

    const { powTargetSpacing, minimumDifficultyBlocks } = this.options;
    const devnetOptions = powTargetSpacing !== undefined || minimumDifficultyBlocks !== undefined
      ? { powTargetSpacing, minimumDifficultyBlocks }
      : undefined;
    this.setSpvChain(new SpvChain(this.options.network, 100, devnetOptions));
  }

  /**
   * @param {BlockHeadersReader} blockHeadersReader instance
   */
  setBlockHeadersReader(blockHeadersReader) {
    this.blockHeadersReader = blockHeadersReader;
  }

  /**
   *
   * @param {SpvChain} spvChain instance
   */
  setSpvChain(spvChain) {
    if (this.spvChain) {
      this.spvChain.removeListener(SpvChain.EVENTS.CONNECTED, this.chainConnectedHandler);
    }
    this.spvChain = spvChain;
    this.spvChain.on(SpvChain.EVENTS.CONNECTED, this.chainConnectedHandler);
  }

  /**
   * @private
   */
  async init() {
    await SpvChain.wasmX11Ready();

    if (!this.blockHeadersReader) {
      this.blockHeadersReader = new BlockHeadersReader(
        {
          coreMethods: this.coreMethods,
          maxParallelStreams: this.options.maxParallelStreams,
          targetBatchSize: this.options.targetBatchSize,
          maxRetries: this.options.maxRetries,
        },
        this.createHistoricalSyncStream,
        this.createContinuousSyncStream,
      );
    }

    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.headersHandler);
    this.blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, this.errorHandler);
  }

  destroyReader() {
    this.removeReaderListeners();
    this.blockHeadersReader = null;
  }

  /**
   * Initializes SPV chain with a list of headers and a known lastSyncedHeaderHeight
   * @param {BlockHeader[]} headers array of headers
   * @param firstHeaderHeight {number} first block header height
   */
  async initializeChainWith(headers, firstHeaderHeight) {
    await SpvChain.wasmX11Ready();

    if (headers.length === 0) {
      // If there are no headers, initialize chain from genesis
      this.spvChain.initialize();
    } else {
      this.spvChain.initialize(headers[0], firstHeaderHeight);
      this.spvChain.addHeaders(headers);
    }
  }

  /**
   * Checks whether spv chain has header at specified height and flushes chains if not
   * @private
   * @param {number} height block height
   */
  ensureChainRoot(height) {
    // Flush spv chain in case header at specified height was not found
    if (!this.spvChain.hashesByHeight.has(height - 1)) {
      this.spvChain.reset();
      this.spvChain.pendingStartBlockHeight = height;
    }
  }

  /**
   * Reads historical block headers
   * @param {number} fromBlockHeight height block height
   * @param {number} toBlockHeight height block height
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight, toBlockHeight) {
    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not read historical data while being in ${this.state} state.`);
    }

    await this.init();

    this.ensureChainRoot(fromBlockHeight);

    this.blockHeadersReader
      .once(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, this.historicalDataObtainedHandler);

    await this.blockHeadersReader.readHistorical(
      fromBlockHeight,
      toBlockHeight,
    );

    this.state = STATES.HISTORICAL_SYNC;
    this.emit(EVENTS.HISTORICAL_SYNC_STARTED);
  }

  async startContinuousSync(fromBlockHeight) {
    if (this.state !== STATES.IDLE) {
      throw new Error(`BlockHeaderProvider can not sync continuous data while being in ${this.state} state.`);
    }

    await this.init();
    this.ensureChainRoot(fromBlockHeight);
    this.continuousLookback = 1;
    await this.blockHeadersReader.subscribeToNew(
      fromBlockHeight,
      () => this.getContinuousResumeHeight(),
    );
    this.state = STATES.CONTINUOUS_SYNC;
    this.emit(EVENTS.CONTINUOUS_SYNC_STARTED);
  }

  async stop() {
    if (this.state === STATES.IDLE) {
      return;
    }

    if (this.state === STATES.HISTORICAL_SYNC) {
      this.blockHeadersReader.stopReadingHistorical();
    } else if (this.state === STATES.CONTINUOUS_SYNC) {
      this.blockHeadersReader.unsubscribeFromNew();
    }

    this.destroyReader();

    this.state = STATES.IDLE;

    this.emit(EVENTS.STOPPED);
  }

  errorHandler(e) {
    this.destroyReader();
    this.emit(EVENTS.ERROR, e);
  }

  /**
   * @private
   * @param {object} headersData object with header and headHeight
   * @param {function} reject callback function
   */
  headersHandler(headersData, reject) {
    const { headers, headHeight } = headersData;

    // TODO: simulate headers rejection and debug
    // BlockHeadersReader might not correctly set startBlockHeight
    // which results in orphan chunks in SPV

    try {
      // Headers that reach the main chain come back through chainConnectedHandler
      this.spvChain.addHeaders(headers, headHeight);

      if (this.state === STATES.CONTINUOUS_SYNC && this.spvChain.getOrphanChunks().length) {
        // Their parents were never sent: ask again from further below the tip
        this.spvChain.dropOrphans();
        this.continuousLookback = Math.min(
          this.continuousLookback * 2,
          this.spvChain.confirmsBeforeFinal + 1,
        );
        reject(new SPVError(`New headers don't connect to the chain, requesting them again from height ${this.getContinuousResumeHeight()}`));
      }
    } catch (e) {
      if (e instanceof SPVError) {
        reject(e);
      } else {
        this.errorHandler(e);
      }
    }
  }

  /**
   * @private
   * @param {BlockHeader[]} headers - the main chain from firstHeight on;
   *   after a reorg firstHeight is at or below the previous tip
   * @param {number} firstHeight
   */
  chainConnectedHandler(headers, firstHeight) {
    this.continuousLookback = 1;
    this.emit(EVENTS.CHAIN_UPDATED, headers, firstHeight);
  }

  /**
   * @private
   * @return {number}
   */
  getContinuousResumeHeight() {
    return Math.max(1, this.spvChain.getTipHeight() - this.continuousLookback + 1);
  }

  historicalDataObtainedHandler() {
    try {
      this.spvChain.validate();
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      this.removeReaderListeners();
    } catch (e) {
      this.errorHandler(e);
    } finally {
      this.state = STATES.IDLE;
    }
  }

  removeReaderListeners() {
    this.blockHeadersReader.removeListener(BlockHeadersReader.EVENTS.ERROR, this.errorHandler);
    this.blockHeadersReader
      .removeListener(BlockHeadersReader.EVENTS.BLOCK_HEADERS, this.headersHandler);
    this.blockHeadersReader
      .removeListener(
        BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED,
        this.historicalDataObtainedHandler,
      );
  }
}

BlockHeadersProvider.EVENTS = EVENTS;
BlockHeadersProvider.STATES = STATES;
BlockHeadersProvider.defaultOptions = { ...defaultOptions };

module.exports = BlockHeadersProvider;

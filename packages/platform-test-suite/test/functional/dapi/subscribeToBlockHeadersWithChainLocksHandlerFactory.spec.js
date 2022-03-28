const EventEmitter = require('events');
const DAPIClient = require('@dashevo/dapi-client');
const {
  Block,
  BlockHeader,
  ChainLock,
} = require('@dashevo/dashcore-lib');

const getDAPISeeds = require('../../../lib/test/getDAPISeeds');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

const wait = (ms) => new Promise((resolve) => {
  setTimeout(resolve, ms);
});

const createRetryableStream = (dapiClient) => {
  const streamMediator = new EventEmitter();

  const maxRetries = 10;
  let currentRetries = 0;

  const createStream = async (fromBlockHeight, count = 0) => {
    let streamError;
    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks(
      {
        fromBlockHeight,
        count,
      },
    );

    streamMediator.cancel = stream.cancel;

    stream.on('data', (data) => {
      streamMediator.emit('data', data);
    });

    stream.on('error', (e) => {
      streamError = e;
      if (currentRetries === maxRetries) {
        streamMediator.emit('error', e);
        return;
      }

      createStream(fromBlockHeight, count)
        .then(() => {
          currentRetries++;
        })
        .catch((createStreamError) => {
          streamMediator.emit('error', createStreamError);
        });
    });

    stream.on('end', () => {
      if (!streamError) {
        streamMediator.emit('end');
      }
    });
  };
  streamMediator.init = createStream;

  return streamMediator;
};

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let dapiClient;
  let sdkClient;
  const network = process.env.NETWORK;

  let bestBlock;
  let bestBlockHeight;

  beforeEach(async () => {
    dapiClient = new DAPIClient({
      network,
      seeds: getDAPISeeds(),
    });

    const bestBlockHash = await dapiClient.core.getBestBlockHash();
    bestBlock = new Block(
      await dapiClient.core.getBlockByHash(bestBlockHash),
    );
    bestBlockHeight = bestBlock.transactions[0].extraPayload.height;
  });

  after(async () => {
    await sdkClient.disconnect();
  });

  it('should respond with only historical data', async () => {
    const headersAmount = 10;
    const historicalBlockHeaders = [];
    let bestChainLock = null;

    const stream = createRetryableStream(dapiClient);
    await stream.init(1, headersAmount);

    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        blockHeaders.getHeadersList().forEach((header) => {
          historicalBlockHeaders.push(new BlockHeader(Buffer.from(header, 'hex')));
        });
      }

      const rawChainLock = data.getChainLock();

      if (rawChainLock) {
        bestChainLock = new ChainLock(Buffer.from(rawChainLock));
      }
    });

    let streamEnded = false;

    stream.on('end', () => {
      streamEnded = true;
    });

    let streamError;
    stream.on('error', (e) => {
      streamError = e;
    });

    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }
      await wait(1000);
    }
    expect(streamError).to.not.exist();
    expect(streamEnded).to.be.true();

    // TODO: fetching blocks one by one takes too long. Implement getBlockHeaders in dapi-client
    const fetchedBlocks = [];

    for (let i = 1; i <= headersAmount; i++) {
      const rawBlock = await dapiClient.core.getBlockByHeight(i);
      const block = new Block(rawBlock);

      fetchedBlocks.push(block);
    }

    expect(historicalBlockHeaders.map((header) => header.hash))
      .to.deep.equal(fetchedBlocks.map((block) => block.header.hash));
    expect(bestChainLock.height).to.exist();
  });

  it('should respond with both new and historical data', async () => {
    let latestChainLock = null;

    const historicalBlocksToGet = 10;
    const blockHeadersHashesFromStream = new Set();

    let obtainedFreshBlock = false;

    sdkClient = await createClientWithFundedWallet();
    const account = await sdkClient.getWalletAccount();
    // Connect to the stream
    const stream = createRetryableStream(dapiClient);
    await stream.init(bestBlockHeight - historicalBlocksToGet + 1);

    let streamEnded = false;
    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        const list = blockHeaders.getHeadersList();
        list.forEach((headerBytes) => {
          const header = new BlockHeader(Buffer.from(headerBytes));
          blockHeadersHashesFromStream.add(header.hash);
          // Once we've obtained a required amount of historical blocks,
          // we can consider the rest arriving as newly generated
          if (blockHeadersHashesFromStream.size > historicalBlocksToGet) {
            obtainedFreshBlock = true;
          }
        });
      }

      const rawChainLock = data.getChainLock();
      if (rawChainLock) {
        latestChainLock = new ChainLock(Buffer.from(rawChainLock));
      }

      if (obtainedFreshBlock && latestChainLock) {
        stream.cancel();
        streamEnded = true;
      }
    });

    let streamError;
    stream.on('error', (e) => {
      streamError = e;
    });

    stream.on('end', () => {
      streamEnded = true;
    });

    // Create and broadcast transaction to produce fresh block
    const transaction = account.createTransaction({
      recipient: account.getUnusedAddress().address,
      satoshis: 1000,
    });

    await dapiClient.core.broadcastTransaction(transaction.toBuffer());
    // Wait for stream ending
    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }

      await wait(1000);
    }

    expect(streamError).to.not.exist();

    // TODO: fetching blocks one by one takes too long. Implement getBlockHeaders in dapi-client
    const fetchedHistoricalBlocks = [];

    for (let i = bestBlockHeight - historicalBlocksToGet + 1; i <= bestBlockHeight; i++) {
      const rawBlock = await dapiClient.core.getBlockByHeight(i);
      const block = new Block(rawBlock);

      fetchedHistoricalBlocks.push(block);
    }

    for (let i = 0; i < historicalBlocksToGet; i++) {
      expect(fetchedHistoricalBlocks[i].header.hash).to.equal([...blockHeadersHashesFromStream][i]);
    }

    expect(obtainedFreshBlock).to.be.true();
    expect(latestChainLock).to.exist();
  });
});

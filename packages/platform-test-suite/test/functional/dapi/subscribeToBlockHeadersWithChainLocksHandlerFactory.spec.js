const DAPIClient = require('@dashevo/dapi-client');
const {
  Block,
  BlockHeader,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const getDAPISeeds = require('../../../lib/test/getDAPISeeds');

const wait = (ms) => new Promise((resolve) => {
  setTimeout(resolve, ms);
});

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let dapiClient;
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

  it('should respond with only historical data', async () => {
    const headersAmount = 10;
    const historicalBlockHeaders = [];
    let bestChainLockSignature = null;

    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight: 1,
      count: headersAmount,
    });

    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        blockHeaders.getHeadersList().forEach((header) => {
          historicalBlockHeaders.push(BlockHeader.fromBuffer(Buffer.from(header)));
        });
      }

      const clsSigMessages = data.getChainLockSignatureMessages();

      if (clsSigMessages) {
        [bestChainLockSignature] = clsSigMessages.getMessagesList();
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

    expect(streamEnded).to.be.true();

    // TODO: fetching blocks one by one takes too long. Implement getBlockHeaders in dapi-client
    const fetchedBlocks = await Promise.all(
      Array.from({ length: headersAmount })
        .map(async (_, index) => new Block(await dapiClient.core.getBlockByHeight(index + 1))),
    );

    expect(historicalBlockHeaders.map((header) => header.hash))
      .to.deep.equal(fetchedBlocks.map((block) => block.header.hash));
    expect(bestChainLockSignature).to.exist();
  });

  it('should respond with both new and historical data', async () => {
    const blocksToGenerate = 5;
    const clSigsToAcquire = 1;
    const numHistoricalBlocks = 10;
    const blockHeadersHashesFromStream = [];
    const blockHeadersHashesGenerated = [];

    let numClSigsAcquired = 0;
    let allHeadersSettled = false;

    // Connect to the stream
    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks(
      {
        fromBlockHeight: bestBlockHeight - numHistoricalBlocks + 1,
      },
    );

    let streamEnded = false;
    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        const list = blockHeaders.getHeadersList();
        list.forEach((headerBytes) => {
          const header = new BlockHeader(Buffer.from(headerBytes));
          blockHeadersHashesFromStream.push(header.hash);
        });

        allHeadersSettled = blockHeadersHashesGenerated.length >= blocksToGenerate
          && blockHeadersHashesGenerated
            .every((hash) => blockHeadersHashesFromStream.includes(hash));
      }

      const clsSigMessages = data.getChainLockSignatureMessages();

      if (clsSigMessages && clsSigMessages.getMessagesList().length > 0) {
        numClSigsAcquired++;
      }

      if (allHeadersSettled && numClSigsAcquired === clSigsToAcquire) {
        stream.destroy();
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

    // Generate blocks
    while (blockHeadersHashesGenerated.length < blocksToGenerate) {
      const address = new PrivateKey().toAddress(network).toString();
      const blockHash = (await dapiClient.core.generateToAddress(1, address))[0];
      const block = new Block(await dapiClient.core.getBlockByHash(blockHash));
      blockHeadersHashesGenerated.push(block.header.hash);
      await wait(500);
    }

    // Wait for stream ending
    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }

      await wait(1000);
    }

    // TODO: fetching blocks one by one takes too long. Implement getBlockHeaders in dapi-client
    const fetchedHistoricalBlocks = await Promise.all(
      Array.from({ length: numHistoricalBlocks })
        .map(async (_, index) => {
          const height = bestBlockHeight - numHistoricalBlocks + index + 1;
          return new Block(await dapiClient.core.getBlockByHeight(height));
        }),
    );

    for (let i = 0; i < numHistoricalBlocks; i++) {
      expect(fetchedHistoricalBlocks[i].header.hash).to.equal(blockHeadersHashesFromStream[i]);
    }

    expect(allHeadersSettled).to.be.true();
    expect(numClSigsAcquired).to.equal(clSigsToAcquire);
  });
});

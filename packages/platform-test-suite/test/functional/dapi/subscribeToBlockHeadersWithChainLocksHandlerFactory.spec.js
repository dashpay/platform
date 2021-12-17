const DAPIClient = require('@dashevo/dapi-client');
const {
  Block,
  BlockHeader,
} = require('@dashevo/dashcore-lib');

const getDAPISeeds = require('../../../lib/test/getDAPISeeds');

const wait = (ms) => new Promise((resolve) => {
  setTimeout(resolve, ms);
});

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let dapiClient;
  const historicalBlockHeaders = [];

  let bestBlockHeight;

  before(async () => {
    dapiClient = new DAPIClient({
      network: process.env.NETWORK,
      seeds: getDAPISeeds(),
    });

    const bestBlockHash = await dapiClient.core.getBestBlockHash();
    const bestBlock = new Block(
      await dapiClient.core.getBlockByHash(bestBlockHash),
    );
    bestBlockHeight = bestBlock.transactions[0].extraPayload.height;
  });

  it('should respond with only historical data', async () => {
    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight: 1,
      count: bestBlockHeight,
    });

    stream.on('data', (data) => {
      data.getBlockHeaders().getHeadersList().forEach((header) => {
        historicalBlockHeaders.push(new BlockHeader(Buffer.from(header)));
      });
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

    // TODO: implement more sophisticated of checking the blocks
    expect(historicalBlockHeaders.length).to.be.equal(bestBlockHeight);
  });
});

const DAPIClient = require('@dashevo/dapi-client');
const getDAPISeeds = require('../../../lib/test/getDAPISeeds');

describe('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let dapiClient;
  beforeEach(() => {
    dapiClient = new DAPIClient({
      network: process.env.NETWORK,
      seeds: getDAPISeeds(),
    });
  });

  it('should', async () => {
    // console.log('Hello', dapiClient);
    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks();
    stream.on('data', (_) => _);

    stream.on('error', (_) => _);

    stream.on('end', () => {});

    await new Promise((resolve) => {
      setTimeout(resolve, 1000);
    });
  });
});

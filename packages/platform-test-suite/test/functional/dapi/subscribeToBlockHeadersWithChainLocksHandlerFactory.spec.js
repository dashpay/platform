import EventEmitter from 'events';
import Dash from 'dash';

import GrpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';
import getDAPISeeds from '../../../lib/test/getDAPISeeds.js';
import createClientWithFundedWallet from '../../../lib/test/createClientWithFundedWallet.js';

const {
  Core: {
    BlockHeader,
    ChainLock,
  },
  DAPIClient,
} = Dash;

const wait = (ms) => new Promise((resolve) => {
  setTimeout(resolve, ms);
});
// TODO: rework with ReconnectableStream
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

    streamMediator.cancel = stream.cancel.bind(stream);

    stream.on('data', (data) => {
      streamMediator.emit('data', data);
    });

    stream.on('error', (e) => {
      if (e.code === GrpcErrorCodes.CANCELLED) {
        streamMediator.emit('end');
        return;
      }

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

  let bestBlockHeight;

  beforeEach(async () => {
    dapiClient = new DAPIClient({
      network,
      seeds: getDAPISeeds(),
    });

    bestBlockHeight = await dapiClient.core.getBestBlockHeight();
  });

  after(async () => {
    if (sdkClient) {
      await sdkClient.disconnect();
    }
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

    // TODO: Use promise instead of loop
    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }
      await wait(1000);
    }
    expect(streamError).to.not.exist();
    expect(streamEnded).to.be.true();

    expect(historicalBlockHeaders.length).to.equal(headersAmount);
    expect(bestChainLock.height).to.exist();
  });

  it('should respond with both new and historical data', async () => {
    let latestChainLock = null;

    const historicalBlocksToGet = 10;
    const blockHeadersHashesFromStream = new Set();

    let obtainedFreshBlock = false;

    sdkClient = await createClientWithFundedWallet(200000);
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

    await account.broadcastTransaction(transaction);

    // TODO: Use promise instead of loop
    // Wait for stream ending
    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }

      await wait(1000);
    }

    expect(streamError).to.not.exist();

    expect(obtainedFreshBlock).to.be.true();
    expect(latestChainLock).to.exist();
  });
});

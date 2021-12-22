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
  const historicalBlockHeaders = [];

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

  it('should respond with only new data', async () => {
    const blocksToGenerate = 10;
    const blockHeadersHashesFromStream = [];
    // const generatedBlockHeaderHash = '';

    // Connect to the stream
    const stream = await dapiClient.core.subscribeToBlockHeadersWithChainLocks(
      {
        fromBlockHeight: bestBlockHeight,
      },
    );

    let streamEnded = false;
    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        const list = blockHeaders.getHeadersList();
        list.forEach((header) => {
          blockHeadersHashesFromStream.push(new BlockHeader(Buffer.from(header)).hash);
        });

        // TODO: come up with more sophisticated way of checking block headers
        if (blockHeadersHashesFromStream.length >= blocksToGenerate + 1) {
          stream.destroy();
          streamEnded = true;
        }
      }
    });

    let streamError;
    stream.on('error', (e) => {
      streamError = e;
    });

    stream.on('end', () => {
      streamEnded = true;
    });

    const fundingPK = new PrivateKey();
    const fundingAddress = fundingPK.toAddress(network).toString();
    await dapiClient.core.generateToAddress(blocksToGenerate, fundingAddress);

    while (!streamEnded) {
      if (streamError) {
        throw streamError;
      }

      await wait(1000);
    }

    // const fundingBlockHash = (await dapiClient.core.generateToAddress(10, fundingAddress))[0];
    // const fundingBlock = new Block(await dapiClient.core.getBlockByHash(fundingBlockHash));
    // const coinbaseTx = fundingBlock.transactions[0];
    //
    // const newAddress = new PrivateKey().toAddress(network).toString();
    // const newTx = new Transaction()
    //   .from(new Transaction.UnspentOutput({
    //     address: fundingAddress,
    //     txId: coinbaseTx.hash,
    //     outputIndex: 0,
    //     script: coinbaseTx.outputs[0].script,
    //     satoshis: coinbaseTx.outputs[0].satoshis,
    //   }))
    //   .to(newAddress, 10000)
    //   .change(fundingAddress)
    //   .fee(668)
    //   .sign(fundingPK);
    //
    // const newTxHash = await dapiClient.core.broadcastTransaction(newTx.toBuffer());
    //
    // let { confirmations, blockHash } = await dapiClient.core.getTransaction(newTxHash);
    //
    // // Wait for transaction to settle in the block
    // // console.log(blockHash.byteLength);
    // while (confirmations === 0) {
    //   ({ confirmations, blockHash } = await dapiClient.core.getTransaction(newTxHash));
    //   await wait(1000);
    // }
    //
    // const newTxBlock =
    // new Block(await dapiClient.core.getBlockByHash(Buffer.from(blockHash).toString('hex')));
    // generatedBlockHeaderHash = newTxBlock.header.hash;
    // console.log(blockHeadersHashesFromStream);
    // console.log(newTxBlock.header.hash);
    // console.log(blockHeadersHashesFromStream.includes(newTxBlock.header.hash));
    // console.log(newTxBlock.header);
    // await wait(20000);
    //
    // if (streamEnded) {
    //   throw new Error('Stream has ended');
    // }
    //
    // if (streamError) {
    //   throw streamError;
    // }

    // stream.destroy();
  });
});

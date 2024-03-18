const { expect } = require('chai');
const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');

const TransactionsReader = require('./TransactionsReader');
const TxStreamMock = require('../../../test/mocks/TxStreamMock');
const { createBloomFilter } = require('./utils');
const { mockMerkleBlock } = require('../../../test/mocks/dashcore/block');
const { mockInstantLock } = require('../../../test/mocks/dashcore/instantlock');
const { waitOneTick } = require('../../../test/utils');

describe('TransactionsReader - unit', () => {
  let options;

  let transactionsReader;
  let historicalSyncStream;
  let continuousSyncStream;

  const MAX_RETRIES = 3;
  const CHAIN_HEIGHT = 1000;
  const NETWORK = 'testnet';

  const DEFAULT_ADDRESSES = ['yfLBwbdPKpKd7bSZ9ABrzTiknu67nDMqTJ', 'yYcL6ezfPgUWNV8fEp2gkw69ArDn76vus2', 'yfLBwbdPKpKd7bSZ9ABrzTiknu67nDMqTJ'];

  beforeEach(function () {
    options = {
      network: NETWORK,
      maxRetries: 3,
    };

    const createHistoricalSyncStream = this.sinon.stub()
      .callsFake(async () => {
        const streamMock = new TxStreamMock(this.sinon);
        historicalSyncStream = streamMock;
        return streamMock;
      });

    const createContinuousSyncStream = this.sinon.stub()
      .callsFake(async () => {
        const streamMock = new TxStreamMock(this.sinon);
        continuousSyncStream = streamMock;
        return streamMock;
      });

    transactionsReader = new TransactionsReader(
      options,
      createHistoricalSyncStream,
      createContinuousSyncStream,
    );

    this.sinon.spy(transactionsReader, 'emit');
    this.sinon.spy(transactionsReader, 'on');
    this.sinon.spy(transactionsReader, 'stopHistoricalSync');
    this.sinon.spy(transactionsReader, 'createSubscribeToHistoricalBatch');
    this.sinon.spy(transactionsReader, 'startContinuousSync');
  });

  describe('#createSubscribeToHistoricalBatch', () => {
    let subscribeToHistoricalBatch;
    beforeEach(() => {
      subscribeToHistoricalBatch = transactionsReader.createSubscribeToHistoricalBatch(
        MAX_RETRIES,
      );
    });

    context('Initialization', () => {
      it('should subscribe to transactions stream and hook on events', async () => {
        await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        expect(transactionsReader.createHistoricalSyncStream)
          .to.have.been.calledWith(createBloomFilter(DEFAULT_ADDRESSES), {
            fromBlockHeight: 1,
            count: CHAIN_HEIGHT,
          });

        expect(historicalSyncStream.on).to.have.been.calledWith('data');
        expect(historicalSyncStream.on).to.have.been.calledWith('error');
        expect(historicalSyncStream.on).to.have.been.calledWith('end');
      });
    });

    context('On "data"', () => {
      context('Transactions', () => {
        it('should process transactions', async () => {
          await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(DEFAULT_ADDRESSES[0], 1000),
            new Transaction().to(DEFAULT_ADDRESSES[1], 2000),
          ];
          historicalSyncStream.sendTransactions(transactions);

          const { firstCall } = transactionsReader.emit;
          expect(transactionsReader.emit).to.have.been.calledOnce();
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS);
          expect(firstCall.args[1].map((tx) => tx.hash))
            .to.deep.equal(transactions.map((tx) => tx.hash));
        });

        it('should ignore false positive transactions', async () => {
          await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(new PrivateKey().toAddress(), 1000),
          ];

          historicalSyncStream.sendTransactions(transactions);
          expect(transactionsReader.emit).to.have.not.been.called();
        });
      });

      context('MerkleBlock', () => {
        it('should process merkle block', async () => {
          await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          const merkleBlock = mockMerkleBlock([]);
          historicalSyncStream.sendMerkleBlock(merkleBlock);

          const { firstCall } = transactionsReader.emit;
          expect(transactionsReader.emit).to.have.been.calledOnce();
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.MERKLE_BLOCK);
          const { merkleBlock: emittedMerkleBlock } = firstCall.args[1];
          expect(emittedMerkleBlock.header.hash).to.equal(merkleBlock.header.hash);
        });

        it('should manage acceptMerkleBlock and rejectMerkleBlock callbacks', async () => {
          await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          let firstAccepted = false;
          let failedRejectionError = null;
          let failedAcceptanceError = null;
          transactionsReader.on(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            ({ acceptMerkleBlock, rejectMerkleBlock }) => {
              if (!firstAccepted) {
                firstAccepted = true;
                acceptMerkleBlock(1, []);
                try {
                  rejectMerkleBlock(new Error('test'));
                } catch (e) {
                  failedRejectionError = e;
                }
              } else {
                rejectMerkleBlock(new Error('test'));
                try {
                  acceptMerkleBlock(1, []);
                } catch (e) {
                  failedAcceptanceError = e;
                }
              }
            },
          );

          const merkleBlock = mockMerkleBlock([]);
          historicalSyncStream.sendMerkleBlock(merkleBlock);
          historicalSyncStream.sendMerkleBlock(merkleBlock);

          expect(failedRejectionError.message)
            .to.equal('Unable to reject accepted merkle block');
          expect(failedAcceptanceError.message)
            .to.equal('Unable to accept rejected merkle block');
        });

        context('Merkle Block accepted (Bloom Filter expansion)', () => {
          let fromBlockHeight;
          let merkleBlockHeight;
          let count;
          let merkleBlock;
          let newAddresses;

          beforeEach(() => {
            fromBlockHeight = 100;
            merkleBlockHeight = 300;
            count = CHAIN_HEIGHT - fromBlockHeight + 1;
            merkleBlock = mockMerkleBlock([]);
            newAddresses = [
              'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
              'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
            ];
          });

          it('should restart stream in case new addresses were generated', async () => {
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, newAddresses);
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledTwice();
            const { secondCall } = transactionsReader.createHistoricalSyncStream;

            const newStream = await secondCall.returnValue;

            expect(secondCall.args).to.deep.equal([
              createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]),
              {
                fromBlockHeight: merkleBlockHeight + 1, // Reconnect from the next merkle block
                count: count - merkleBlockHeight + fromBlockHeight - 1, // Adjust remaining count
              },
            ]);

            expect(transactionsReader.historicalSyncStream).to.equal(newStream);
          });

          it('should not restart stream in case no new addresses were generated', async () => {
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, []);
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
          });

          it('should not restart stream for the last merkle block in range in case new addresses were generated', async () => {
            merkleBlockHeight = 1000;
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, newAddresses);
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
          });

          it('should handle stream restart error', async () => {
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            const restartError = new Error('Error restarting stream');
            transactionsReader.createHistoricalSyncStream.throws(restartError);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, newAddresses);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledTwice();
            expect(emittedError).to.equal(restartError);
          });

          it('should throw an error if invalid Merkle Block height provided', async () => {
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            let errorThrown = null;
            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                try {
                  acceptMerkleBlock(1300, newAddresses);
                } catch (e) {
                  errorThrown = e;
                }
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
            expect(errorThrown.message)
              .to.equal('Merkle block height is greater than expected range: 1300 > 1000');
          });
        });

        context('Merkle Block rejected', () => {
          let fromBlockHeight;
          let count;
          const rejectedMerkleBlockWith = new Error('Merkle block rejected with error');
          let merkleBlock;

          beforeEach(() => {
            fromBlockHeight = 100;
            count = CHAIN_HEIGHT - fromBlockHeight + 1;
            merkleBlock = mockMerkleBlock([]);
          });

          it('should cancel and restart stream if Merkle Block rejected', async () => {
            await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ rejectMerkleBlock }) => {
                rejectMerkleBlock(rejectedMerkleBlockWith);
              });

            const { historicalSyncStream: stream } = transactionsReader;
            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();
            expect(historicalSyncStream).to.equal(transactionsReader.historicalSyncStream);
            expect(stream.cancel).to.have.been.calledOnce();
            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledTwice();
          });
        });
      });
    });

    context('On "error"', () => {
      let fromBlockHeight;
      let merkleBlockHeight;
      let merkleBlock;
      let newAddresses;

      beforeEach(() => {
        fromBlockHeight = 100;
        merkleBlockHeight = 300;
        merkleBlock = mockMerkleBlock([]);
        newAddresses = [
          'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
          'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
        ];
      });

      it('should handle stream cancellation', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, CHAIN_HEIGHT, DEFAULT_ADDRESSES);
        await historicalSyncStream.cancel();
        expect(transactionsReader.emit)
          .to.have.not.been.calledWith(TransactionsReader.EVENTS.ERROR);
      });

      it('should retry in case of an error', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(merkleBlockHeight, []);
          });

        historicalSyncStream.sendMerkleBlock(merkleBlock);

        historicalSyncStream.emit('error', new Error('Fake stream error'));

        await waitOneTick();

        // Ensure that retry logic does not re-fetch already processed data
        const blocksRead = merkleBlockHeight - fromBlockHeight + 1;
        const remainingCount = CHAIN_HEIGHT - blocksRead;
        expect(transactionsReader.createHistoricalSyncStream.secondCall.args)
          .to.deep.equal([createBloomFilter(DEFAULT_ADDRESSES), {
            fromBlockHeight: merkleBlockHeight + 1,
            count: remainingCount,
          }]);
      });

      it('should retry with the restartArgs provided by merkle block', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(merkleBlockHeight, newAddresses);
          });

        historicalSyncStream.sendMerkleBlock(merkleBlock);

        await waitOneTick();

        historicalSyncStream.emit('error', new Error('Fake stream error'));

        await waitOneTick();

        // Ensure that retry logic does not re-fetch already processed data
        const blocksRead = merkleBlockHeight - fromBlockHeight + 1;
        const remainingCount = CHAIN_HEIGHT - blocksRead;
        expect(transactionsReader.createHistoricalSyncStream.secondCall.args)
          .to.deep.equal([createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]), {
            fromBlockHeight: merkleBlockHeight + 1,
            count: remainingCount,
          }]);
      });

      it('should not retry if no headers remaining', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(fromBlockHeight + CHAIN_HEIGHT - 1);
          });

        historicalSyncStream.sendMerkleBlock(merkleBlock);

        historicalSyncStream.emit('error', new Error('Fake stream error'));

        await waitOneTick();

        expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
      });

      it('should emit error in case retry attempts were exhausted', async () => {
        await subscribeToHistoricalBatch(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        transactionsReader.on('error', () => {});

        const { maxRetries } = options;
        for (let i = 0; i < maxRetries; i += 1) {
          const error = new Error(`Retry exhaust error ${i}`);
          historicalSyncStream.emit('error', error);
          // eslint-disable-next-line no-await-in-loop
          await waitOneTick();
        }

        const lastError = new Error('Retry exhaust error last');
        historicalSyncStream.emit('error', lastError);

        expect(transactionsReader.emit)
          .to.have.been.calledWith('error', lastError);
      });
    });

    context('On "end"', () => {
      let fromBlockHeight;
      let count;
      let merkleBlockHeight;
      let merkleBlock;
      let newAddresses;

      beforeEach(() => {
        fromBlockHeight = 100;
        count = CHAIN_HEIGHT - fromBlockHeight + 1;
        merkleBlock = mockMerkleBlock([]);
        merkleBlockHeight = 300;
        newAddresses = [
          'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
          'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
        ];
      });

      it('should emit HISTORICAL_DATA_OBTAINED event', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

        historicalSyncStream.end();

        expect(transactionsReader.emit)
          .to.have.been.calledWith(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
      });

      it('should not emit HISTORICAL_DATA_OBTAINED event if stream ended, but needs to be restarted', async () => {
        await subscribeToHistoricalBatch(fromBlockHeight, count, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(merkleBlockHeight, newAddresses);
            historicalSyncStream.end();
          });

        historicalSyncStream.sendMerkleBlock(merkleBlock);
        await waitOneTick();

        expect(transactionsReader.emit)
          .to.have.not.been.calledWith(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
      });
    });
  });

  describe('#startHistoricalSync', () => {
    let fromBlockHeight;
    let toBlockHeight;
    let count;

    beforeEach(() => {
      fromBlockHeight = 1;
      toBlockHeight = CHAIN_HEIGHT;
      count = toBlockHeight - fromBlockHeight + 1;
    });

    it('should start historical sync and subscribe to events', async () => {
      await transactionsReader
        .startHistoricalSync(fromBlockHeight, toBlockHeight, DEFAULT_ADDRESSES);

      expect(transactionsReader.historicalSyncStream).to.exist();
      expect(transactionsReader.createHistoricalSyncStream)
        .to.have.been.calledOnceWith(createBloomFilter(DEFAULT_ADDRESSES), {
          fromBlockHeight,
          count,
        });
    });

    it('should validate arguments', async () => {
      await expect(transactionsReader.startHistoricalSync(2, 1))
        .to.be.rejectedWith('No addresses to sync');

      await expect(transactionsReader.startHistoricalSync(2, 1, []))
        .to.be.rejectedWith('No addresses to sync');

      await expect(transactionsReader.startHistoricalSync(0, 1, DEFAULT_ADDRESSES))
        .to.be.rejectedWith('Invalid fromBlockHeight: 0');

      await expect(transactionsReader.startHistoricalSync(2, 1, DEFAULT_ADDRESSES))
        .to.be.rejectedWith('Invalid total amount of blocks to sync: 0');
    });

    it('should not allow multiple executions', async () => {
      await transactionsReader.startHistoricalSync(1, 2, DEFAULT_ADDRESSES);
      await expect(transactionsReader.startHistoricalSync(1, 2, DEFAULT_ADDRESSES))
        .to.be.rejectedWith('Historical sync is already in process');
    });
  });

  describe('#stopHistoricalSync', () => {
    it('should stop historical sync', async () => {
      await transactionsReader.startHistoricalSync(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);
      expect(transactionsReader.historicalSyncStream).to.exist();
      await transactionsReader.stopHistoricalSync();
      expect(historicalSyncStream.cancel).to.have.been.calledOnce();
      expect(transactionsReader.emit).to.have.been.calledWith(TransactionsReader.EVENTS.STOPPED);
      expect(transactionsReader.historicalSyncStream).to.equal(null);
    });
  });

  describe('#startContinuousSync', () => {
    const fromBlockHeight = 100;

    context('Initialization', async () => {
      it('should initialize continuous sync stream', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        expect(transactionsReader.createContinuousSyncStream)
          .to.have.been.calledWith(createBloomFilter(DEFAULT_ADDRESSES), {
            fromBlockHeight,
            count: 0,
          });

        expect(continuousSyncStream.on).to.have.been.calledWith('data');
        expect(continuousSyncStream.on).to.have.been.calledWith('error');
        expect(continuousSyncStream.on).to.have.been.calledWith('end');
        expect(continuousSyncStream.on).to.have.been.calledWith('beforeReconnect');
      });

      it('should validate arguments', async () => {
        await expect(transactionsReader.startContinuousSync(-1, DEFAULT_ADDRESSES))
          .to.be.rejectedWith('Invalid fromBlockHeight: -1');

        await expect(transactionsReader.startContinuousSync(fromBlockHeight))
          .to.be.rejectedWith('Invalid addresses: undefined');

        await expect(transactionsReader.startContinuousSync(fromBlockHeight, []))
          .to.be.rejectedWith('Empty addresses list provided');
      });

      it('should not allow subscribe twice', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        await expect(transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES))
          .to.be.rejectedWith('Continuous sync has already been started');
      });
    });

    context('On "data"', () => {
      context('Transactions', () => {
        it('should process transactions', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(DEFAULT_ADDRESSES[0], 1000),
            new Transaction().to(DEFAULT_ADDRESSES[1], 2000),
          ];
          continuousSyncStream.sendTransactions(transactions);

          const { firstCall } = transactionsReader.emit;
          expect(transactionsReader.createContinuousSyncStream).to.have.been.calledOnce();
          expect(transactionsReader.emit).to.have.been.calledOnce();
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.NEW_TRANSACTIONS);
          expect(firstCall.args[1].transactions.map((tx) => tx.hash))
            .to.deep.equal(transactions.map((tx) => tx.hash));
          expect(firstCall.args[1].handleNewAddresses)
            .to.be.instanceof(Function);
        });

        it('should ignore false positive transactions', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(new PrivateKey().toAddress(), 1000),
          ];

          continuousSyncStream.sendTransactions(transactions);
          expect(transactionsReader.emit).to.have.not.been.called();
        });

        context('Bloom filter expansion', () => {
          const newAddresses = [
            'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
            'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
          ];

          // This test checks failsafe logic of bloom filter expansion
          // In general, we are expanding bloom filters after instant locks arrival.
          // If for some reason instant lock was delayed or missed, we expand bloom filter
          // when the next batch of transactions is received
          it('should trigger fail-safe mechanism to expand bloom filter in case it wasnt triggered by IS lock', async () => {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            // Handle new transactions
            transactionsReader
              .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ handleNewAddresses }) => {
                // and simulate new addresses generation in response
                handleNewAddresses(newAddresses);
              });

            // Sending a TX. This should trigger EVENTS.NEW_TRANSACTIONS
            // and reader will memorize addresses generated
            continuousSyncStream.sendTransactions([
              new Transaction({}).to(DEFAULT_ADDRESSES[0], 1000),
            ]);
            await waitOneTick();

            // Sending second TX, it should trigger failsafe mechanism for
            // bloom filter expansion
            continuousSyncStream.sendTransactions([
              new Transaction({}).to(DEFAULT_ADDRESSES[1], 1000),
            ]);
            await waitOneTick();

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledTwice();
            const { secondCall } = transactionsReader.createContinuousSyncStream;

            const newStream = await secondCall.returnValue;

            // Tx reader must not process second batch of transactions
            // in case of failsafe logic. It will restart stream, and process these transactions
            // after
            expect(transactionsReader.emit)
              .to.have.been.calledOnce();

            expect(secondCall.args).to.deep.equal([
              createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]),
              {
                fromBlockHeight, // Reconnect
                count: 0,
              },
            ]);

            expect(transactionsReader.continuousSyncStream).to.equal(newStream);
          });

          it('should handle stream restart error in fail-safe mechanism', async () => {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            const restartError = new Error('Error restarting stream');
            transactionsReader.createContinuousSyncStream.throws(restartError);

            transactionsReader
              .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ handleNewAddresses }) => {
                handleNewAddresses(newAddresses);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            const transactions = [
              new Transaction({}).to(DEFAULT_ADDRESSES[1], 1000),
            ];
            // Send first batch of transactions to update generated addresses
            continuousSyncStream.sendTransactions(transactions);
            await waitOneTick();

            // Send first batch of transactions to trigger bloom filter restart
            continuousSyncStream.sendTransactions(transactions);
            await waitOneTick();

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledTwice();
            expect(emittedError).to.equal(restartError);
          });
        });
      });

      context('MerkleBlock', () => {
        it('should process merkle block', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          const merkleBlock = mockMerkleBlock([]);
          continuousSyncStream.sendMerkleBlock(merkleBlock);

          expect(transactionsReader.emit).to.have.been.calledOnce();
          const { firstCall } = transactionsReader.emit;
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.MERKLE_BLOCK);
          const { merkleBlock: emittedMerkleBlock } = firstCall.args[1];
          expect(emittedMerkleBlock.header.hash).to.equal(merkleBlock.header.hash);
        });

        it('should manage acceptMerkleBlock and rejectMerkleBlock callbacks', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          let firstAccepted = false;
          let failedRejectionError = null;
          let failedAcceptanceError = null;
          transactionsReader.on('error', () => {});

          transactionsReader.on(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            ({ acceptMerkleBlock, rejectMerkleBlock }) => {
              if (!firstAccepted) {
                firstAccepted = true;
                acceptMerkleBlock(fromBlockHeight + 1, []);
                try {
                  rejectMerkleBlock(new Error('Rejected'));
                } catch (e) {
                  failedRejectionError = e;
                }
              } else {
                rejectMerkleBlock(new Error('Rejected'));
                try {
                  acceptMerkleBlock(1, []);
                } catch (e) {
                  failedAcceptanceError = e;
                }
              }
            },
          );

          const merkleBlock = mockMerkleBlock([]);
          continuousSyncStream.sendMerkleBlock(merkleBlock);
          continuousSyncStream.sendMerkleBlock(merkleBlock);

          expect(failedRejectionError.message)
            .to.equal('Unable to reject accepted merkle block');
          expect(failedAcceptanceError.message)
            .to.equal('Unable to accept rejected merkle block');
        });

        context('Merkle Block accepted', () => {
          let merkleBlockHeight;
          let merkleBlock;

          beforeEach(() => {
            merkleBlockHeight = CHAIN_HEIGHT + 1;
            merkleBlock = mockMerkleBlock([]);
          });

          it('should accept merkle block', async () => {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight);
              });

            const transactions = [
              new Transaction({}).to(DEFAULT_ADDRESSES[0], 1000),
            ];

            continuousSyncStream.sendTransactions(transactions);
            await waitOneTick();
            continuousSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.emit)
              .to.have.not.been.calledWith('error');
          });

          it('should throw an error if invalid Merkle Block height provided', async function () {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            continuousSyncStream.retryOnError = this.sinon.stub()
              .callsFake((e) => {
                continuousSyncStream.emit('error', e);
              });

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(fromBlockHeight - 1);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            continuousSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledOnce();
            expect(emittedError.message)
              .to.equal('Merkle block height is lesser than expected startBlockHeight: 99 < 100');
          });
        });

        context('Merkle Block rejected', () => {
          const rejectedMerkleBlockWith = new Error('Merkle block rejected with error');
          let merkleBlock;

          beforeEach(() => {
            merkleBlock = mockMerkleBlock([]);
          });

          it('should emit error if Merkle Block rejected', async function () {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            continuousSyncStream.retryOnError = this.sinon.stub()
              .callsFake((e) => {
                continuousSyncStream.emit('error', e);
              });

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ rejectMerkleBlock }) => {
                rejectMerkleBlock(rejectedMerkleBlockWith);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            continuousSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(emittedError)
              .to.equal(rejectedMerkleBlockWith);
          });
        });
      });

      context('Instant locks', () => {
        it('should process instant lock', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          const instantLock = mockInstantLock(Buffer.alloc(32).toString('hex'));
          continuousSyncStream.sendISLocks([instantLock]);

          expect(transactionsReader.emit).to.have.been.calledOnce();
          const { firstCall } = transactionsReader.emit;
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.INSTANT_LOCKS);
          const emittedInstantLocks = firstCall.args[1];
          expect(emittedInstantLocks).to.deep.equal([instantLock]);
        });

        context('Bloom filter expansion', () => {
          const newAddresses = [
            'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
            'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
          ];

          it('should expand Bloom filter in case new addresses were generated by TX', async () => {
            await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

            // Handle new transactions
            transactionsReader
              .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ handleNewAddresses }) => {
                // and simulate new addresses generation in response
                handleNewAddresses(newAddresses);
              });

            const tx = new Transaction({}).to(DEFAULT_ADDRESSES[0], 1000);
            // Sending a TX. This should trigger EVENTS.NEW_TRANSACTIONS
            // and reader will memorize addresses generated
            continuousSyncStream.sendTransactions([tx]);
            await waitOneTick();

            // Sending a corresponding instant lock. It should trigger stream restart
            // with expanded bloom filter
            const instantLock = mockInstantLock(tx.hash);
            continuousSyncStream.sendISLocks([instantLock]);
            await waitOneTick();

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledTwice();
            const { secondCall } = transactionsReader.createContinuousSyncStream;

            const newStream = await secondCall.returnValue;

            expect(secondCall.args).to.deep.equal([
              createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]),
              {
                fromBlockHeight, // Reconnect
                count: 0,
              },
            ]);

            expect(transactionsReader.continuousSyncStream).to.equal(newStream);
          });
        });
      });
    });

    context('On "error', () => {
      it('should emit error', async () => {
        await transactionsReader.startContinuousSync(1, DEFAULT_ADDRESSES);

        let emittedError = null;
        transactionsReader.on('error', (e) => {
          emittedError = e;
        });

        const error = new Error('Error');
        continuousSyncStream.emit('error', error);

        await waitOneTick();

        expect(emittedError).to.equal(error);
      });

      it('should handle stream cancellation', async () => {
        await transactionsReader
          .startContinuousSync(1, DEFAULT_ADDRESSES);
        await continuousSyncStream.cancel();
        expect(transactionsReader.startContinuousSync).to.have.been.calledOnce();
        expect(transactionsReader.emit)
          .to.have.not.been.calledWith(TransactionsReader.EVENTS.ERROR);
      });
    });

    context('On "end"', () => {
      it('should end stream', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        continuousSyncStream.end();

        expect(transactionsReader.continuousSyncStream).to.equal(null);
      });
    });

    context('On "beforeReconnect"', () => {
      it('should reconnect with the same args if no new merkle block was fetched', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        let newArgs = null;
        continuousSyncStream.emit('beforeReconnect', (...updatedArgs) => {
          newArgs = updatedArgs;
        });

        expect(newArgs).to.deep.equal([
          createBloomFilter(DEFAULT_ADDRESSES),
          {
            fromBlockHeight,
            count: 0,
          },
        ]);
      });

      it('should update fromBlockHeight in case merkle block was fetched', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(fromBlockHeight + 1);
          });

        continuousSyncStream.sendMerkleBlock(mockMerkleBlock([]));

        let newArgs = null;
        continuousSyncStream.emit('beforeReconnect', (...updatedArgs) => {
          newArgs = updatedArgs;
        });

        expect(newArgs).to.deep.equal([
          createBloomFilter(DEFAULT_ADDRESSES),
          {
            fromBlockHeight: fromBlockHeight + 1,
            count: 0,
          },
        ]);
      });
    });
  });

  describe('#stopContinuousSync', () => {
    it('should stop continuous sync', async () => {
      await transactionsReader.startContinuousSync(CHAIN_HEIGHT, DEFAULT_ADDRESSES);
      expect(transactionsReader.continuousSyncStream).to.exist();
      await transactionsReader.stopContinuousSync();
      expect(continuousSyncStream.cancel).to.have.been.calledOnce();
      expect(transactionsReader.emit).to.have.been.calledWith(TransactionsReader.EVENTS.STOPPED);
      expect(transactionsReader.continuousSyncStream).to.equal(null);
    });
  });
});

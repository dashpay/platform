const { expect } = require('chai');
const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');

const TransactionsReader = require('./TransactionsReader');
const TxStreamMock = require('../../../test/mocks/TxStreamMock');
const { createBloomFilter } = require('./utils');
const mockMerkleBlock = require('../../../test/mocks/mockMerkleBlock');
const { waitOneTick } = require('../../../test/utils');

describe('TransactionsReader - unit', () => {
  let options;

  let transactionsReader;
  let historicalSyncStream;
  let continuousSyncStream;

  const CHAIN_HEIGHT = 1000;
  const NETWORK = 'testnet';

  const DEFAULT_ADDRESSES = ['yfLBwbdPKpKd7bSZ9ABrzTiknu67nDMqTJ', 'yYcL6ezfPgUWNV8fEp2gkw69ArDn76vus2', 'yfLBwbdPKpKd7bSZ9ABrzTiknu67nDMqTJ'];

  beforeEach(function () {
    options = {
      network: NETWORK,
      createHistoricalSyncStream: () => {},
      createContinuousSyncStream: () => {},
    };

    this.sinon.stub(options, 'createHistoricalSyncStream')
      .callsFake(async () => {
        const streamMock = new TxStreamMock(this.sinon);
        historicalSyncStream = streamMock;
        return streamMock;
      });

    this.sinon.stub(options, 'createContinuousSyncStream')
      .callsFake(async () => {
        const streamMock = new TxStreamMock(this.sinon);
        continuousSyncStream = streamMock;
        return streamMock;
      });

    transactionsReader = new TransactionsReader(options);
    this.sinon.spy(transactionsReader, 'emit');
    this.sinon.spy(transactionsReader, 'on');
    this.sinon.spy(transactionsReader, 'stopReadingHistorical');
    this.sinon.spy(transactionsReader, 'subscribeToHistoricalStream');
    this.sinon.spy(transactionsReader, 'startContinuousSync');
  });

  describe('#subscribeToHistoricalStream', () => {
    context('Initialization', () => {
      it('should subscribe to transactions stream and hook on events', async () => {
        await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

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
          await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

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
          await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(new PrivateKey().toAddress(), 1000),
          ];

          historicalSyncStream.sendTransactions(transactions);
          expect(transactionsReader.emit).to.have.not.been.called();
        });
      });

      context('MerkleBlock', () => {
        it('should process merkle block', async () => {
          await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

          const merkleBlock = mockMerkleBlock([]);
          historicalSyncStream.sendMerkleBlock(merkleBlock);

          const { firstCall } = transactionsReader.emit;
          expect(transactionsReader.emit).to.have.been.calledOnce();
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.MERKLE_BLOCK);
          const { merkleBlock: emittedMerkleBlock } = firstCall.args[1];
          expect(emittedMerkleBlock.header.hash).to.equal(merkleBlock.header.hash);
        });

        it('should manage acceptMerkleBlock and rejectMerkleBlock callbacks', async () => {
          await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

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
                  rejectMerkleBlock();
                } catch (e) {
                  failedRejectionError = e;
                }
              } else {
                rejectMerkleBlock();
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
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

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
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, []);
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
          });

          it('should not restart stream for the last merkle block in range in case new addresses were generated', async () => {
            merkleBlockHeight = 1000;
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight, newAddresses);
              });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
          });

          it('should handle stream restart error', async () => {
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

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
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(1300, newAddresses);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledOnce();
            expect(emittedError.message)
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

          it('should emit error if Merkle Block rejected', async () => {
            await transactionsReader.subscribeToHistoricalStream(
              fromBlockHeight, count, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ rejectMerkleBlock }) => {
                rejectMerkleBlock(rejectedMerkleBlockWith);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            historicalSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(emittedError)
              .to.equal(rejectedMerkleBlockWith);
          });
        });
      });
    });

    context('On "error"', () => {
      it('should emit error', async () => {
        await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        let emittedError = null;
        transactionsReader.on('error', (e) => {
          emittedError = e;
        });

        const error = new Error('Error');
        historicalSyncStream.emit('error', error);

        await waitOneTick();

        expect(emittedError).to.equal(error);
      });

      it('should handle stream cancellation', async () => {
        await transactionsReader
          .subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);
        await historicalSyncStream.cancel();
        expect(transactionsReader.subscribeToHistoricalStream).to.have.been.calledOnce();
        expect(transactionsReader.historicalSyncStream).to.equal(null);
        expect(transactionsReader.emit)
          .to.have.not.been.calledWith(TransactionsReader.EVENTS.ERROR);
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
        await transactionsReader.subscribeToHistoricalStream(
          fromBlockHeight, count, DEFAULT_ADDRESSES,
        );

        historicalSyncStream.end();

        expect(transactionsReader.emit)
          .to.have.been.calledWith(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
      });

      it('should not emit HISTORICAL_DATA_OBTAINED event if stream ended, but needs to be restarted', async () => {
        await transactionsReader.subscribeToHistoricalStream(
          fromBlockHeight, count, DEFAULT_ADDRESSES,
        );

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
          expect(transactionsReader.emit).to.have.been.calledOnce();
          expect(firstCall.args[0]).to.equal(TransactionsReader.EVENTS.NEW_TRANSACTIONS);
          expect(firstCall.args[1].transactions.map((tx) => tx.hash))
            .to.deep.equal(transactions.map((tx) => tx.hash));
          expect(firstCall.args[1].appendAddresses)
            .to.be.instanceof(Function);
        });

        it('should ignore false positive transactions', async () => {
          await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

          const transactions = [
            new Transaction().to(new PrivateKey().toAddress(), 1000),
          ];

          historicalSyncStream.sendTransactions(transactions);
          expect(transactionsReader.emit).to.have.not.been.called();
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
          transactionsReader.on(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            ({ acceptMerkleBlock, rejectMerkleBlock }) => {
              if (!firstAccepted) {
                firstAccepted = true;
                acceptMerkleBlock(fromBlockHeight + 1, []);
                try {
                  rejectMerkleBlock();
                } catch (e) {
                  failedRejectionError = e;
                }
              } else {
                rejectMerkleBlock();
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

        context('Merkle Block accepted (Bloom Filter expansion)', () => {
          let merkleBlockHeight;
          let merkleBlock;
          let newAddresses;

          beforeEach(() => {
            merkleBlockHeight = CHAIN_HEIGHT + 1;
            merkleBlock = mockMerkleBlock([]);
            newAddresses = [
              'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
              'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
            ];
          });

          it('should restart stream in case new addresses were generated', async () => {
            await transactionsReader.startContinuousSync(
              fromBlockHeight, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ appendAddresses }) => {
                appendAddresses(newAddresses);
              });

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

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledTwice();
            const { secondCall } = transactionsReader.createContinuousSyncStream;

            const newStream = await secondCall.returnValue;

            expect(secondCall.args).to.deep.equal([
              createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]),
              {
                fromBlockHeight: merkleBlockHeight + 1, // Reconnect from the next merkle block
                count: 0,
              },
            ]);

            expect(transactionsReader.continuousSyncStream).to.equal(newStream);
          });

          it('should not restart stream in case no new addresses were generated', async () => {
            await transactionsReader.startContinuousSync(
              fromBlockHeight, DEFAULT_ADDRESSES,
            );

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight);
              });

            continuousSyncStream.sendMerkleBlock(merkleBlock);

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledOnce();
          });

          it('should handle stream restart error', async () => {
            await transactionsReader.startContinuousSync(
              fromBlockHeight, DEFAULT_ADDRESSES,
            );

            const restartError = new Error('Error restarting stream');
            transactionsReader.createContinuousSyncStream.throws(restartError);

            transactionsReader
              .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ appendAddresses }) => {
                appendAddresses(newAddresses);
              });

            transactionsReader
              .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
                acceptMerkleBlock(merkleBlockHeight);
              });

            let emittedError = null;
            transactionsReader.on('error', (e) => {
              emittedError = e;
            });

            const transactions = [
              new Transaction({}).to(DEFAULT_ADDRESSES[0], 1000),
            ];
            continuousSyncStream.sendTransactions(transactions);
            await waitOneTick();
            continuousSyncStream.sendMerkleBlock(merkleBlock);
            await waitOneTick();

            expect(transactionsReader.createContinuousSyncStream).to.have.been.calledTwice();
            expect(emittedError).to.equal(restartError);
          });

          it('should throw an error if invalid Merkle Block height provided', async () => {
            await transactionsReader.startContinuousSync(
              fromBlockHeight, DEFAULT_ADDRESSES,
            );

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

          it('should emit error if Merkle Block rejected', async () => {
            await transactionsReader.startContinuousSync(
              fromBlockHeight, DEFAULT_ADDRESSES,
            );

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
        expect(transactionsReader.continuousSyncStream).to.equal(null);
        expect(transactionsReader.emit)
          .to.have.not.been.calledWith(TransactionsReader.EVENTS.ERROR);
      });
    });

    context('On "end"', () => {
      it('should end stream', async () => {
        await transactionsReader.startContinuousSync(
          fromBlockHeight, DEFAULT_ADDRESSES,
        );

        continuousSyncStream.end();

        expect(transactionsReader.continuousSyncStream).to.equal(null);
      });
    });

    context('On "beforeReconnect"', () => {
      let merkleBlock;
      let newAddresses;

      beforeEach(() => {
        merkleBlock = mockMerkleBlock([]);
        newAddresses = [
          'XcPmHAafCTrXe15auqobQkMrqMhwCt6KkC',
          'XeTVfNCZVzLSFvPBXuKRE1R8XVjgKKwUy8',
        ];
      });

      it('should reconnect with the same args if no new merkle block was fetched', async () => {
        await transactionsReader.startContinuousSync(
          fromBlockHeight, DEFAULT_ADDRESSES,
        );

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
            fromBlockHeight: fromBlockHeight + 2,
            count: 0,
          },
        ]);
      });

      it('should not update args in case new addresses were generated and stream restarted', async () => {
        await transactionsReader.startContinuousSync(fromBlockHeight, DEFAULT_ADDRESSES);

        transactionsReader
          .on(TransactionsReader.EVENTS.NEW_TRANSACTIONS, ({ appendAddresses }) => {
            appendAddresses(newAddresses);
          });

        transactionsReader
          .on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(fromBlockHeight + 1);
          });

        const transactions = [
          new Transaction({}).to(DEFAULT_ADDRESSES[0], 1000),
        ];
        continuousSyncStream.sendTransactions(transactions);
        await waitOneTick();
        continuousSyncStream.sendMerkleBlock(merkleBlock);

        let newArgs = null;
        continuousSyncStream.emit('beforeReconnect', (...updatedArgs) => {
          newArgs = updatedArgs;
        });

        expect(newArgs).to.equal(null);
      });
    });
  });
});

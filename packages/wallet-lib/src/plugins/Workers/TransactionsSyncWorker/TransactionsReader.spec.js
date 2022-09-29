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
  });

  describe('#subscribeToHistoricalStream', () => {
    context('initialization', () => {
      it('should subscribe to transactions stream and hook on events', async () => {
        await transactionsReader.subscribeToHistoricalStream(1, CHAIN_HEIGHT, DEFAULT_ADDRESSES);

        expect(transactionsReader.createHistoricalSyncStream)
          .to.have.been.calledWith(1, CHAIN_HEIGHT, createBloomFilter(DEFAULT_ADDRESSES));

        expect(historicalSyncStream.on).to.have.been.calledWith('data');
        expect(historicalSyncStream.on).to.have.been.calledWith('error');
        expect(historicalSyncStream.on).to.have.been.calledWith('end');
      });
    });

    context('[data event] Transactions', () => {
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
    context('[data event] MerkleBlock', () => {
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

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
            acceptMerkleBlock(merkleBlockHeight, newAddresses);
          });

          historicalSyncStream.sendMerkleBlock(merkleBlock);
          await waitOneTick();

          expect(transactionsReader.createHistoricalSyncStream).to.have.been.calledTwice();
          const { secondCall } = transactionsReader.createHistoricalSyncStream;

          const newStream = await secondCall.returnValue;

          expect(secondCall.args).to.deep.equal([
            merkleBlockHeight + 1, // Reconnect from the next merkle block
            count - merkleBlockHeight + fromBlockHeight - 1, // Adjust remaining count
            createBloomFilter([...DEFAULT_ADDRESSES, ...newAddresses]),
          ]);

          expect(transactionsReader.historicalSyncStream).to.equal(newStream);
        });

        it('should not restart stream in case no new addresses were generated', async () => {
          await transactionsReader.subscribeToHistoricalStream(
            fromBlockHeight, count, DEFAULT_ADDRESSES,
          );

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
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

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
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

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
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

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ acceptMerkleBlock }) => {
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

          transactionsReader.on(TransactionsReader.EVENTS.MERKLE_BLOCK, ({ rejectMerkleBlock }) => {
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
});

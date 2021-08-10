const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const { Transaction } = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
    stream: {
      AcknowledgingWritable,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
    RawTransactions,
    BloomFilter,
  },
} = require('@dashevo/dapi-grpc');

const { BloomFilter: CoreBloomFilter } = require('@dashevo/dashcore-lib');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const subscribeToTransactionsWithProofsHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/tx-filter-stream/subscribeToTransactionsWithProofsHandlerFactory',
);

const ProcessMediator = require('../../../../../lib/transactionsFilter/ProcessMediator');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('subscribeToTransactionsWithProofsHandlerFactory', () => {
  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  let call;
  let subscribeToTransactionsWithProofsHandler;
  let bloomFilterEmitterCollectionMock;
  let historicalTxData;
  let getHistoricalTransactionsIteratorMock;
  let subscribeToNewTransactionsMock;
  let testTransactionAgainstFilterMock;
  let coreAPIMock;
  let getMemPoolTransactionsMock;

  beforeEach(function beforeEach() {
    const bloomFilterMessage = new BloomFilter();

    bloomFilterMessage.setVData(new Uint8Array());
    bloomFilterMessage.setNTweak(1000);
    bloomFilterMessage.setNFlags(100);
    bloomFilterMessage.setNHashFuncs(10);

    const request = new TransactionsWithProofsRequest();

    request.setBloomFilter(bloomFilterMessage);

    call = new GrpcCallMock(this.sinon, request);

    bloomFilterEmitterCollectionMock = {
      test: this.sinon.stub(),
    };

    historicalTxData = [];
    getHistoricalTransactionsIteratorMock = this.sinon.spy(function* generator() {
      for (let i = 0; i < historicalTxData.length; i++) {
        yield historicalTxData[i];
      }
    });

    subscribeToNewTransactionsMock = this.sinon.stub();
    testTransactionAgainstFilterMock = this.sinon.stub();

    coreAPIMock = {
      getBlock: this.sinon.stub(),
      getBestBlockHeight: this.sinon.stub(),
      getBlockHash: this.sinon.stub(),
    };

    getMemPoolTransactionsMock = this.sinon.stub().returns([]);

    subscribeToTransactionsWithProofsHandler = subscribeToTransactionsWithProofsHandlerFactory(
      getHistoricalTransactionsIteratorMock,
      subscribeToNewTransactionsMock,
      bloomFilterEmitterCollectionMock,
      testTransactionAgainstFilterMock,
      coreAPIMock,
      getMemPoolTransactionsMock,
    );

    this.sinon.spy(ProcessMediator.prototype, 'emit');
    this.sinon.spy(ProcessMediator.prototype, 'on');
  });

  it('should respond with error if bloom filter is not valid', async () => {
    const bloomFilterMessage = new BloomFilter();

    bloomFilterMessage.setVData(new Uint8Array());
    bloomFilterMessage.setNTweak(1000);
    bloomFilterMessage.setNFlags(100);
    bloomFilterMessage.setNHashFuncs(100);

    const request = new TransactionsWithProofsRequest();

    request.setBloomFilter(bloomFilterMessage);

    call.request = request;

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid bloom filter: '
        + '"nHashFuncs" exceeded max size "50"');

      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
      expect(getMemPoolTransactionsMock).to.not.have.been.called();
    }
  });

  it('should respond with error if `fromBlockHeight is 0 and `fromBlockHash` is not set', async () => {
    const bloomFilterMessage = new BloomFilter();

    bloomFilterMessage.setVData(new Uint8Array());
    bloomFilterMessage.setNTweak(1000);
    bloomFilterMessage.setNFlags(100);
    bloomFilterMessage.setNHashFuncs(10);

    const request = new TransactionsWithProofsRequest();

    request.setFromBlockHeight(0);
    request.setBloomFilter(bloomFilterMessage);

    call.request = request;

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('should fail with InvalidArgumentGrpcError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('minimum value for `fromBlockHeight` is 1');
      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
      expect(getMemPoolTransactionsMock).to.not.have.been.called();
    }
  });

  it('should respond with error if if both fromBlockHash and fromBlockHeight are not specified', async () => {
    const bloomFilterMessage = new BloomFilter();

    bloomFilterMessage.setVData(new Uint8Array());
    bloomFilterMessage.setNTweak(1000);
    bloomFilterMessage.setNFlags(100);
    bloomFilterMessage.setNHashFuncs(10);

    const request = new TransactionsWithProofsRequest();

    request.setBloomFilter(bloomFilterMessage);

    call.request = request;

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('should fail with InvalidArgumentGrpcError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('minimum value for `fromBlockHeight` is 1');
      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
      expect(getMemPoolTransactionsMock).to.not.have.been.called();
    }
  });

  it('should respond with error if requested data length exceeded blockchain length', async () => {
    call.request.setFromBlockHash('someBlockHash');
    call.request.setCount(100);

    coreAPIMock.getBlock.resolves({ height: 1 });
    coreAPIMock.getBestBlockHeight.resolves(10);

    try {
      await subscribeToTransactionsWithProofsHandler(call);
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal(
        'count is too big, could not fetch more than blockchain length',
      );

      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
      expect(getMemPoolTransactionsMock).to.not.have.been.called();
    }
  });

  it('should subscribe to new transactions if count is not specified', async function it() {
    call.request.setFromBlockHash('someBlockHash');
    call.request.setCount(0);

    const writableStub = this.sinon.stub(AcknowledgingWritable.prototype, 'write');

    coreAPIMock.getBlock.resolves({ height: 1 });
    coreAPIMock.getBestBlockHeight.resolves(10);

    historicalTxData.push({
      merkleBlock: {
        toBuffer: () => Buffer.from('someHash'),
        header: {
          hash: 'someHash',
        },
      },
      transactions: [
        {
          toBuffer: () => Buffer.from(
            'edefad1c70ee6736a0a0c2f9be7f22cfcf77ae2c120704a98cdc9aebdab7ffc5', 'hex',
          ),
        },
      ],
    });

    const memPoolTransaction = new Transaction(Buffer.from('03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff1703f06a101299dbcd32279d9e01e508000000002f4e614effffffff0285464209000000001976a9146a341485a9444b35dc9cb90d24e7483de7d37e0088ac7f464209000000001976a914ad037df64c0d0ec5d0395eb9a543f93fcc26092388ac00000000260100f06a1000c69a125eeb5ce6fa55c48966174a90253a79ce3350ccc4918ba2cb1463513c88', 'hex'));

    getMemPoolTransactionsMock.returns([memPoolTransaction]);

    await subscribeToTransactionsWithProofsHandler(call);

    const filter = new CoreBloomFilter({
      vData: new Uint8Array([]),
      nTweak: 1000,
      nFlags: 100,
      nHashFuncs: 10,
    });

    expect(getHistoricalTransactionsIteratorMock).to.have.been
      .calledOnceWith(
        filter,
        Buffer.from('someBlockHash').toString('hex'),
        10,
      );
    expect(getMemPoolTransactionsMock).to.have.been.calledOnce();

    expect(subscribeToNewTransactionsMock).to.have.been.calledOnce();
    expect(writableStub).to.have.been.calledTwice();

    const firstResponse = new TransactionsWithProofsResponse();
    const rawTransactions = new RawTransactions();
    rawTransactions.setTransactionsList(
      historicalTxData[0].transactions.map(tx => tx.toBuffer()),
    );
    firstResponse.setRawTransactions(rawTransactions);

    const secondResponse = new TransactionsWithProofsResponse();
    secondResponse.setRawMerkleBlock(historicalTxData[0].merkleBlock.toBuffer());

    expect(writableStub.getCall(0).args).to.deep.equal(
      [firstResponse],
    );

    expect(writableStub.getCall(1).args).to.deep.equal(
      [secondResponse],
    );

    expect(bloomFilterEmitterCollectionMock.test).to.be.calledOnceWith(memPoolTransaction);
  });

  it('should end call and emit CLIENT_DISCONNECTED event when client disconnects', async () => {
    call.request.setFromBlockHash('someHash');
    coreAPIMock.getBlock.resolves({ height: 1 });

    await subscribeToTransactionsWithProofsHandler(call);

    // Client disconnects
    call.emit('cancelled');

    // Bloom filters was removed when client disconnects
    expect(ProcessMediator.prototype.emit.getCall(2)).to.be.calledWith(
      ProcessMediator.EVENTS.CLIENT_DISCONNECTED,
    );

    expect(call.write).to.not.have.been.called();
    expect(call.end).to.have.been.calledOnce();
    expect(getMemPoolTransactionsMock).to.have.been.calledOnce();
  });

  it('should respond with Not Found error if fromBlockHeight is bigger than block count', async () => {
    call.request.setFromBlockHeight(100);
    call.request.setCount(0);

    const error = new Error();
    error.code = -8;

    coreAPIMock.getBlockHash.throws(error);

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('should fail with NotFoundGrpcError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);

      expect(e.getMessage()).to.equal(
        'fromBlockHeight is bigger than block count',
      );

      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
    }
  });

  it('should respond with not found error if fromBlockHash is not found', async () => {
    call.request.setFromBlockHash('someBlockHash');
    call.request.setCount(0);

    const error = new Error();
    error.code = -5;

    coreAPIMock.getBlock.throws(error);

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('should fail with NotFoundGrpcError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.getMessage()).to.equal(
        'fromBlockHash is not found',
      );

      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
    }
  });

  it('should respond with Not Found error if fromBlockHash is not part of the best block chain', async () => {
    const blockHash = 'someBlockHash';
    call.request.setFromBlockHash(blockHash);
    call.request.setCount(0);

    const error = new Error();
    error.code = -5;

    coreAPIMock.getBlock.resolves({ height: 1, confirmations: -1 });

    try {
      await subscribeToTransactionsWithProofsHandler(call);

      expect.fail('should throw NotFoundGrpcError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.getMessage()).to.equal(
        `block ${Buffer.from(blockHash).toString('hex')} is not part of the best block chain`,
      );

      expect(call.write).to.not.have.been.called();
      expect(call.end).to.not.have.been.called();
    }
  });
});

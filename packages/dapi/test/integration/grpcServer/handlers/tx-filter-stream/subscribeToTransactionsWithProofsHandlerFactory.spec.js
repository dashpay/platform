const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const {
  Transaction,
  Block,
  BlockHeader,
  MerkleBlock,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const { BloomFilter } = require('@dashevo/dashcore-lib');
const { TransactionFilterResponse } = require('@dashevo/dapi-grpc');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const BloomFilterEmitterCollection = require('../../../../../lib/bloomFilter/emitter/BloomFilterEmitterCollection');
const testTransactionAgainstFilterCollectionFactory = require('../../../../../lib/transactionsFilter/testRawTransactionAgainstFilterCollectionFactory');
const emitBlockEventToFilterCollectionFactory = require('../../../../../lib/transactionsFilter/emitBlockEventToFilterCollectionFactory');
const testTransactionsAgainstFilter = require('../../../../../lib/transactionsFilter/testTransactionAgainstFilter');
const subscribeToTransactionsWithProofsHandlerFactory = require('../../../../../lib/grpcServer/handlers/tx-filter-stream/subscribeToTransactionsWithProofsHandlerFactory');

const InvalidArgumentError = require('../../../../../lib/grpcServer/error/InvalidArgumentError');

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
  let bloomFilterEmitterCollection;
  let emitBlockEventToFilterCollection;
  let testRawTransactionAgainstFilterCollection;
  let transaction;

  beforeEach(function beforeEach() {
    const address = new PrivateKey().toAddress();
    transaction = new Transaction().to(address, 10);

    call = new GrpcCallMock(this.sinon);

    bloomFilterEmitterCollection = new BloomFilterEmitterCollection();
    emitBlockEventToFilterCollection = emitBlockEventToFilterCollectionFactory(
      bloomFilterEmitterCollection,
    );
    testRawTransactionAgainstFilterCollection = testTransactionAgainstFilterCollectionFactory(
      bloomFilterEmitterCollection,
    );

    subscribeToTransactionsWithProofsHandler = subscribeToTransactionsWithProofsHandlerFactory(
      bloomFilterEmitterCollection,
      testTransactionsAgainstFilter,
    );
  });

  it('should respond with error if bloom filter is not valid', function it() {
    // Create a wrong bloom filter
    call.request = {
      nHashFuncs: 100,
      nTweak: 1000,
      nFlags: 100,
      vData: [],
    };

    const callback = this.sinon.stub();

    // Get the bloom filter from a client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Call listener when new transaction appears
    testRawTransactionAgainstFilterCollection(transaction.toBuffer());

    expect(callback).to.have.been.calledOnce();
    expect(callback.getCall(0).args).to.have.lengthOf(2);

    const [error, response] = callback.getCall(0).args;

    expect(error).to.be.instanceOf(InvalidArgumentError);
    expect(error.getMessage()).to.equal('Invalid argument: Invalid bloom filter: '
      + '"nHashFuncs" exceeded max size "50"');

    expect(response).to.be.null();

    expect(call.write).to.not.have.been.called();
    expect(call.end).to.not.have.been.called();
  });

  it('should send a matched raw transaction when it appears', function it() {
    // Create a bloom filter with a transaction hash
    const bloomFilter = BloomFilter.create(1, 0.01);

    const binaryTransactionHash = Buffer.from(transaction.hash, 'hex');

    bloomFilter.insert(binaryTransactionHash);

    call.request = bloomFilter.toObject();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Call listener when a new transaction appears
    testRawTransactionAgainstFilterCollection(transaction.toBuffer());

    const expectedResponse = new TransactionFilterResponse();
    expectedResponse.setRawTransaction(transaction.toBuffer());

    expect(call.write).to.have.been.calledOnceWith(expectedResponse.toObject());
    expect(call.end).to.not.have.been.called();
    expect(callback).to.not.have.been.called();
  });

  it('should not send a not matched raw transaction when it appears', function it() {
    // Create an empty bloom filter
    const bloomFilter = BloomFilter.create(1, 0.01);

    call.request = bloomFilter.toObject();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Call listener when new transaction appears
    testRawTransactionAgainstFilterCollection(transaction.toBuffer());

    expect(call.write).to.not.have.been.called();
    expect(call.end).to.not.have.been.called();
    expect(callback).to.not.have.been.called();
  });

  it('should send a merkle block with sent matched transactions when a new block is mined', function it() {
    // Create a bloom filter with transaction hash
    const bloomFilter = BloomFilter.create(1, 0.01);

    const binaryTransactionHash = Buffer.from(transaction.hash, 'hex');

    bloomFilter.insert(binaryTransactionHash);

    call.request = bloomFilter.toObject();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Call listener when a new transaction appears
    testRawTransactionAgainstFilterCollection(transaction.toBuffer());

    // Create one more transaction which will not match the bloom filter
    const address = new PrivateKey().toAddress();
    const notMatchedTransaction = new Transaction().to(address, 10);

    // Create a block with both transactions
    const blockHeader = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: '1d00dda1',
      nonce: 1449878272,
    });

    const block = new Block({
      header: blockHeader.toObject(),
      transactions: [transaction, notMatchedTransaction],
    });

    // Call listener when a new block appears
    emitBlockEventToFilterCollection(block.toBuffer());

    expect(call.write).to.have.been.calledTwice();

    // Matched transaction must be sent
    const expectedResponse = new TransactionFilterResponse();
    expectedResponse.setRawTransaction(transaction.toBuffer());

    expect(call.write.getCall(0)).to.have.been.calledWith(expectedResponse.toObject());

    // Merkle block with matched transaction hash must be sent
    const rawMerkleBlockResponse = call.write.getCall(1).args[0];
    expect(rawMerkleBlockResponse).to.have.property('rawMerkleBlock');

    // TransactionFilterResponse converts buffers to base64 before send
    const rawMerkleBlock = Buffer.from(rawMerkleBlockResponse.rawMerkleBlock, 'base64');
    const merkleBlock = new MerkleBlock(rawMerkleBlock);

    expect(merkleBlock.hashes).to.have.lengthOf(2);
    expect(merkleBlock.hashes).to.have.members([transaction.hash, notMatchedTransaction.hash]);
    expect(merkleBlock.header.hash).to.equal(blockHeader.hash);

    // noinspection JSAccessibilityCheck
    expect(merkleBlock.hasTransaction(notMatchedTransaction.hash)).to.be.false();
    // noinspection JSAccessibilityCheck
    expect(merkleBlock.hasTransaction(transaction.hash)).to.be.true();

    expect(call.end).to.not.have.been.called();
    expect(callback).to.not.have.been.called();
  });

  it('should not send a merkle block if it doesn\'t contain sent matched transactions', function it() {
    // Create bloom filter with transaction hash
    const bloomFilter = BloomFilter.create(1, 0.01);

    const binaryTransactionHash = Buffer.from(transaction.hash, 'hex');

    bloomFilter.insert(binaryTransactionHash);

    call.request = bloomFilter.toObject();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Call listener when new transaction appears
    testRawTransactionAgainstFilterCollection(transaction.toBuffer());

    // Create one more transaction which will not match the bloom filter
    const address = new PrivateKey().toAddress();
    const notMatchedTransaction = new Transaction().to(address, 10);

    // Create a block with only mot matched transaction
    const blockHeader = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: '1d00dda1',
      nonce: 1449878272,
    });

    const block = new Block({
      header: blockHeader.toObject(),
      transactions: [notMatchedTransaction],
    });

    // Call listener when a new block appears
    emitBlockEventToFilterCollection(block.toBuffer());

    // Matched transaction must be sent
    const expectedResponse = new TransactionFilterResponse();
    expectedResponse.setRawTransaction(transaction.toBuffer());

    expect(call.write).to.have.been.calledOnceWith(expectedResponse.toObject());
    expect(call.end).to.not.have.been.called();
    expect(callback).to.not.have.been.called();
  });

  it('should not send a merkle block if there is no matched transactions', function it() {
    // Create bloom filter with transaction hash
    const bloomFilter = BloomFilter.create(1, 0.01);

    const binaryTransactionHash = Buffer.from(transaction.hash, 'hex');

    bloomFilter.insert(binaryTransactionHash);

    call.request = bloomFilter.toObject();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // Create one more transaction which will not match the bloom filter
    const address = new PrivateKey().toAddress();
    const notMatchedTransaction = new Transaction().to(address, 10);

    // Create a block with both transactions
    const blockHeader = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: '1d00dda1',
      nonce: 1449878272,
    });

    const block = new Block({
      header: blockHeader.toObject(),
      transactions: [transaction, notMatchedTransaction],
    });

    // Call listener when a new block appears
    emitBlockEventToFilterCollection(block.toBuffer());

    // Matched transaction must be sent
    const expectedResponse = new TransactionFilterResponse();
    expectedResponse.setRawTransaction(transaction.toBuffer());

    expect(call.write).to.not.have.been.called();
    expect(call.end).to.not.have.been.called();
    expect(callback).to.not.have.been.called();
  });

  it('should end call and remove the bloom filter emitter from the collection when client disconnects', function it() {
    // Create empty bloom filter
    const bloomFilter = BloomFilter.create(1, 0.01);

    call.request = bloomFilter.toObject();

    // There are no bloom filters yet
    expect(bloomFilterEmitterCollection.filters).to.be.empty();

    const callback = this.sinon.stub();

    // Get the bloom filter from client
    subscribeToTransactionsWithProofsHandler(call, callback);

    // The new bloom filter was added
    expect(bloomFilterEmitterCollection.filters).to.have.lengthOf(1);

    // Client disconnects
    call.emit('cancelled');

    // Bloom filters was removed when client disconnects
    expect(bloomFilterEmitterCollection.filters).to.be.empty();

    expect(call.write).to.not.have.been.called();
    expect(call.end).to.have.been.calledOnce();
    expect(callback).to.have.been.calledOnceWith(null, null);
  });
});

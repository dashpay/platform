const sinon = require('sinon');
const {ChainLock, BlockHeader} = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      NotFoundGrpcError,
      InvalidArgumentGrpcError,
    },
    stream: {
      AcknowledgingWritable,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BlockHeadersWithChainLocksRequest,
    BlockHeadersWithChainLocksResponse,
    BlockHeaders,
  },
} = require('@dashevo/dapi-grpc');
const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');
const subscribeToBlockHeadersWithChainLocksHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToBlockHeadersWithChainLocksHandlerFactory',
);
const getHistoricalBlockHeadersIteratorFactory = require("../../../../../lib/grpcServer/handlers/blockheaders-stream/getHistoricalBlockHeadersIteratorFactory");
const cache = require("../../../../../lib/grpcServer/handlers/blockheaders-stream/cache");

let coreAPIMock;
let zmqClientMock;

describe.only('subscribeToBlockHeadersWithChainLocksHandlerFactory', () => {
  let call;
  let subscribeToBlockHeadersWithChainLocksHandler;
  let getHistoricalBlockHeadersIterator;
  let subscribeToNewBlockHeadersMock;

  const cacheSpy = sinon.spy(cache);

  const writableStub = sinon.stub(AcknowledgingWritable.prototype, 'write');

  beforeEach(function () {
    coreAPIMock = {
      getBlock: sinon.stub(),
      getBlockStats: sinon.stub(),
      getBlockHeaders: sinon.stub(),
      getBestBlockHeight: sinon.stub(),
      getBlockHash: sinon.stub(),
      getBestChainLock: sinon.stub(),
    };

    subscribeToNewBlockHeadersMock = sinon.stub();

    cache.set.resetHistory()
    cache.get.resetHistory()

    async function* asyncGenerator() {
      yield [{toBuffer: () => Buffer.from('fake', 'utf-8')}];
    }

    getHistoricalBlockHeadersIteratorMock = () => asyncGenerator();
    zmqClientMock = {on: sinon.stub(), topics: {hashblock: 'fake'}};

    // eslint-disable-next-line operator-linebreak
    subscribeToBlockHeadersWithChainLocksHandler =
      subscribeToBlockHeadersWithChainLocksHandlerFactory(
        getHistoricalBlockHeadersIteratorMock,
        coreAPIMock,
        zmqClientMock,
        subscribeToNewBlockHeadersMock,
      );
  });

  it('should subscribe to newBlockHeaders', async function () {
    const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');

    let request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(sinon, request);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash,
    });
    coreAPIMock.getBlockStats.resolves({height: 1});

    await subscribeToBlockHeadersWithChainLocksHandler(call);
    expect(subscribeToNewBlockHeadersMock).to.have.been.called();
    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(
      blockHash.toString('hex'),
      ['height'],
    );
  });

  it('should subscribe from block hash', async function () {
    const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');
    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(sinon, request);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakesig', 'hex'),
      blockHash: Buffer.from('fakeHash', 'hex'),
    });

    coreAPIMock.getBlockStats.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(
      blockHash.toString('hex'),
      ['height'],
    );

    const clSigResponse = new BlockHeadersWithChainLocksResponse();
    clSigResponse.setChainLock(new ChainLock({
      height: 1,
      signature: Buffer.from('fakesig', 'hex'),
      blockHash: Buffer.from('fakeHash', 'hex'),
    }).toBuffer());

    expect(writableStub.getCall(0).args).to.deep.equal(
      [clSigResponse],
    );

    const blockHeadersProto = new BlockHeaders();
    blockHeadersProto.setHeadersList(
      [Buffer.from('fake', 'utf-8')],
    );
    const iteratorResponse = new BlockHeadersWithChainLocksResponse();
    iteratorResponse.setBlockHeaders(blockHeadersProto);

    expect(writableStub.getCall(1).args).to.deep.equal(
      [iteratorResponse],
    );
  });

  it('should subscribe from block height', async function () {
    const blockHeight = 1;
    const count = 5;

    let request = new BlockHeadersWithChainLocksRequest();
    request.setFromBlockHeight(blockHeight);
    request.setCount(count);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(sinon, request);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash: Buffer.from('fakeHash'),
    });

    coreAPIMock.getBlockStats.resolves({ height: 1 });

    await subscribeToBlockHeadersWithChainLocksHandler(call);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(
      blockHeight,
      ['height'],
    );

    expect(subscribeToNewBlockHeadersMock).to.not.have.been.called();
  });

  it('should handle getBlockStats RPC method errors', async function () {
    const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');

    let request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHash(blockHash);
    request.setCount(0);

    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(sinon, request);

    try {
      coreAPIMock.getBlockStats.throws({code: -5});

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message).to.be.equal(`Block ${blockHash.toString('hex')} not found`);
    }

    try {
      coreAPIMock.getBlockStats.throws({ code: -8 });

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.message).to.be.equal(`Block ${blockHash.toString('hex')} not found`);
    }

    try {
      request.setCount(10);

      coreAPIMock.getBlockStats.resolves({height: 10});

      coreAPIMock.getBestBlockHeight.resolves(11);

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
    }
  });


  describe('getHistoricalBlockHeaders and cache', function () {
    const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');
    const fakeBlockHeaderHex = '00000020272e374a06c87a0ce0e6ee1a0754c98b9ec2493e7c0ac7ba41a0730000000000568b3c4156090db4d8db5447762e95dd1d4c921c96801a9086720ded85266325916cc05caa94001c5caf3595'
    const differentFakeBlockHeaderHex = '000000202be60663802ead0740cb6d6e49ee7824481280f03c71369eb90f7b00000000006abd277facc8cf02886d88662dbcc2adb6d8de7a491915e74bed4d835656a4f1f26dc05ced93001ccf81cabc'

    let request = new BlockHeadersWithChainLocksRequest();
    request.setFromBlockHash(blockHash);
    request.setCount(0);
    request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

    call = new GrpcCallMock(sinon, request);

    beforeEach(function () {
      historicalBlockHeadersIterator = getHistoricalBlockHeadersIteratorFactory(coreAPIMock)

      subscribeToBlockHeadersWithChainLocksHandler =
        subscribeToBlockHeadersWithChainLocksHandlerFactory(
          historicalBlockHeadersIterator,
          coreAPIMock,
          zmqClientMock,
          subscribeToNewBlockHeadersMock,
        );

      cacheSpy.get.resetHistory()
      cacheSpy.set.resetHistory()
    });


    it('should iterate over getHistoricalBlockHeaders without cache', async function () {
      coreAPIMock.getBestChainLock.resolves({
        height: 3,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestBlockHeight.resolves(3);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders).to.be.calledOnceWithExactly(blockHash.toString('hex'), 3);

      expect(cacheSpy.get.callCount).to.be.equal(3);
      expect(cacheSpy.set.callCount).to.be.equal(3);

      expect(cacheSpy.get).to.always.returned(undefined);

      expect(cacheSpy.set).to.be.calledWith(1, fakeBlockHeaderHex)
      expect(cacheSpy.set).to.be.calledWith(2, fakeBlockHeaderHex)
      expect(cacheSpy.set).to.be.calledWith(3, fakeBlockHeaderHex)
    });

    it('should use cache and do not call for blockHeaders', async function () {
      coreAPIMock.getBestChainLock.resolves({
        height: 3,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestBlockHeight.resolves(3);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      cache.set(1, fakeBlockHeaderHex)
      cache.set(2, fakeBlockHeaderHex)
      cache.set(3, fakeBlockHeaderHex)
      cacheSpy.set.resetHistory()

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(0)

      expect(cacheSpy.get.callCount).to.be.equal(3);
      expect(cacheSpy.set.callCount).to.be.equal(0);

      expect(cacheSpy.get).to.always.returned(fakeBlockHeaderHex);
    });

    it('should use maximum available cache', async function () {
      coreAPIMock.getBestChainLock.resolves({
        height: 3,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestBlockHeight.resolves(3);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      // should use cache and does not hit rpc
      cache.set(1, fakeBlockHeaderHex)
      cache.set(2, fakeBlockHeaderHex)
      cache.set(3, fakeBlockHeaderHex)
      cacheSpy.set.resetHistory()

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(0)

      expect(cacheSpy.get.callCount).to.be.equal(3);
      expect(cacheSpy.set.callCount).to.be.equal(0);

      expect(cacheSpy.get).to.always.returned(fakeBlockHeaderHex);
    });

    it('should call for rpc if theres no cache', async function () {
      coreAPIMock.getBestChainLock.resolves({
        height: 3,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestBlockHeight.resolves(3);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(1)

      expect(cacheSpy.get.callCount).to.be.equal(3);
      expect(cacheSpy.set.callCount).to.be.equal(3);

      expect(cacheSpy.get).to.always.returned(undefined);
    });

    it('should use cache for only 1 block', async function () {
      coreAPIMock.getBestChainLock.resolves({
        height: 3,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestBlockHeight.resolves(3);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      cache.set(1, fakeBlockHeaderHex)
      cacheSpy.set.resetHistory()

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(1)
      expect(coreAPIMock.getBlockHeaders).to.be.calledOnceWithExactly(blockHash);

      expect(cacheSpy.get.callCount).to.be.equal(3);
      expect(cacheSpy.set.callCount).to.be.equal(3);

      expect(cacheSpy.get).to.always.returned(undefined);
    });

    // the case where we have blockHeaders in cache, but missing something in the middle
    // in this case we pick up last cached blockHeader without a gap
    it('should use cache for different ranges and gaps', async function () {
      const fakeBlockHeaderHex = '00000020272e374a06c87a0ce0e6ee1a0754c98b9ec2493e7c0ac7ba41a0730000000000568b3c4156090db4d8db5447762e95dd1d4c921c96801a9086720ded85266325916cc05caa94001c5caf3595'

      const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');

      let request = new BlockHeadersWithChainLocksRequest();
      request.setFromBlockHash(blockHash);
      request.setCount(0);

      request = BlockHeadersWithChainLocksRequest.deserializeBinary(request.serializeBinary());

      call = new GrpcCallMock(sinon, request);


      // 5 blocks in the network
      // 3,4 is missing from cache
      // means we request for 2,3,4,5
      coreAPIMock.getBlockHash.resolves(blockHash.toString('hex'));
      coreAPIMock.getBestChainLock.resolves({
        height: 5,
        signature: Buffer.from('fakesig', 'hex'),
        blockHash: Buffer.from('fakeHash', 'hex'),
      });
      coreAPIMock.getBlockStats.resolves({height: 1});
      coreAPIMock.getBestBlockHeight.resolves(5);
      coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

      cache.set(1, fakeBlockHeaderHex)
      cache.set(2, differentFakeBlockHeaderHex)
      cache.set(3, undefined)
      cache.set(4, undefined)
      cache.set(5, fakeBlockHeaderHex)
      cacheSpy.set.resetHistory()

      await subscribeToBlockHeadersWithChainLocksHandler(call);

      expect(coreAPIMock.getBlockHash).to.be.calledOnceWithExactly(1);
      expect(coreAPIMock.getBlockHeaders).to.be.calledOnceWithExactly(BlockHeader.fromRawBlock(differentFakeBlockHeaderHex).hash, 4);

      expect(cacheSpy.get.callCount).to.be.equal(5);
      expect(cacheSpy.set.callCount).to.be.equal(5);

      expect(cacheSpy.get).to.returned(differentFakeBlockHeaderHex);
    });
  })

});

const sinon = require('sinon');
const ChainDataProvider = require('../../../lib/providers/chainDataProvider');
const blockHeadersCache = require('../../../lib/providers/blockheaders-cache');

describe('ChainDataProvider', async () => {
  const blockHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c', 'hex');
  const fakeBlockHeaderHex = '00000020272e374a06c87a0ce0e6ee1a0754c98b9ec2493e7c0ac7ba41a0730000000000568b3c4156090db4d8db5447762e95dd1d4c921c96801a9086720ded85266325916cc05caa94001c5caf3595'; // eslint-disable-line
  const differentFakeBlockHeaderHex = '000000202be60663802ead0740cb6d6e49ee7824481280f03c71369eb90f7b00000000006abd277facc8cf02886d88662dbcc2adb6d8de7a491915e74bed4d835656a4f1f26dc05ced93001ccf81cabc'; // eslint-disable-line

  const coreAPIMock = sinon.stub();
  const zmqClientMock = sinon.stub();

  const chainDataProvider = new ChainDataProvider(coreAPIMock, zmqClientMock);

  const cacheSpy = sinon.spy(blockHeadersCache);

  await chainDataProvider.init();

  beforeEach(() => {
    blockHeadersCache.purge();
    cacheSpy.set.resetHistory();
    cacheSpy.get.resetHistory();
  });

  it('should call for chainlock on init', async () => {
    await chainDataProvider.init();
    expect(coreAPIMock.getBestChainLock).to.be.calledOnceWithExactly();
  });

  it('should call for rpc on getBlockHeader when cache is empty', async () => {
    await chainDataProvider.getBlockHeader(blockHash);
    expect(coreAPIMock.getBlockHeader).to.be.calledOnceWithExactly();
    expect(cacheSpy.get).to.be.calledWith(blockHash);
    expect(cacheSpy.get).to.always.returned(undefined);
  });

  it('should return cache on getBlockHeader if it is cached', async () => {
    await chainDataProvider.getBlockHeader(blockHash);
    expect(cacheSpy.getBlockHeader.callCount).to.be.equal(3);
  });

  // the case where we request for N blocks, and theres nothing at all in the cache
  it('should call for rpc when nothing is cached', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex,
      fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

    await chainDataProvider.getBlockHeaders(blockHash, 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
    expect(chainDataProvider.getBlockHeaders).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), 5);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(0);

    expect(cacheSpy.get).to.always.returned(undefined);

    expect(cacheSpy.set).to.be.calledWith(1, fakeBlockHeaderHex);
    expect(cacheSpy.set).to.be.calledWith(2, fakeBlockHeaderHex);
    expect(cacheSpy.set).to.be.calledWith(3, fakeBlockHeaderHex);
    expect(cacheSpy.set).to.be.calledWith(4, fakeBlockHeaderHex);
    expect(cacheSpy.set).to.be.calledWith(5, fakeBlockHeaderHex);
  });

  // the case when we request for cached blocks (all N block are cached)
  it('should use cache and do not call for blockHeaders', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });

    blockHeadersCache.set(1, fakeBlockHeaderHex);
    blockHeadersCache.set(2, fakeBlockHeaderHex);
    blockHeadersCache.set(3, fakeBlockHeaderHex);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(blockHash, 3);

    expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(0);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);

    expect(cacheSpy.get.callCount).to.be.equal(3);
    expect(cacheSpy.set.callCount).to.be.equal(0);

    expect(cacheSpy.get).to.always.returned(fakeBlockHeaderHex);
  });

  // the case where we are missing some blocks in the tail
  // f.e. we request for 5 blocks, and what we have in cache is [1,2,3,undefined,undefined]
  // we should call for 3 blocks (3,4,5) and set 2 missing in the cache
  it('should use cache when miss something in the tail', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex,
      fakeBlockHeaderHex]);

    // should use cache and does not hit rpc
    blockHeadersCache.set(1, fakeBlockHeaderHex);
    blockHeadersCache.set(2, fakeBlockHeaderHex);
    blockHeadersCache.set(3, differentFakeBlockHeaderHex);
    blockHeadersCache.set(4, undefined);
    blockHeadersCache.set(5, undefined);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(blockHash.toString('hex'), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
    expect(chainDataProvider.getBlockHeaders).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), 3);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(2);

    expect(cacheSpy.get).to.returned(differentFakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(fakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(undefined);
  });

  // the case when we miss something in the middle
  // f.e we request for 5 blocks, and cache is [1,2,undefined,undefined,5]
  // should take second block as a start point and request for 4 blocks (to the end)
  it('should use cache when missing in the middle', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex,
      fakeBlockHeaderHex, fakeBlockHeaderHex]);

    blockHeadersCache.set(1, fakeBlockHeaderHex);
    blockHeadersCache.set(2, differentFakeBlockHeaderHex);
    blockHeadersCache.set(3, undefined);
    blockHeadersCache.set(4, undefined);
    blockHeadersCache.set(5, fakeBlockHeaderHex);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(blockHash.toString('hex'), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
    expect(chainDataProvider.getBlockHeaders).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), 4);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(4);

    expect(cacheSpy.get).to.returned(fakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(differentFakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(undefined);
  });

  // the case where we have something in the cache, but the first blocks are not
  // f.e. we request for 5 blocks, and cache is [undefined,undefined,3,4,5]
  it('should not use cache when miss something in the beginning', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex,
      fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

    blockHeadersCache.set(1, undefined);
    blockHeadersCache.set(2, undefined);
    blockHeadersCache.set(3, fakeBlockHeaderHex);
    blockHeadersCache.set(4, fakeBlockHeaderHex);
    blockHeadersCache.set(5, fakeBlockHeaderHex);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(blockHash.toString('hex'), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
    expect(coreAPIMock.getBlockHeaders).to.be
      .calledOnceWithExactly(blockHash.toString('hex'), 5);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(5);

    expect(cacheSpy.get).to.returned(fakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(undefined);
  });

  // the same as above, but with additional gap
  // [undefined,2,undefined,4,5]
  it('should not use cache when miss something in the beginning', async () => {
    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([fakeBlockHeaderHex, fakeBlockHeaderHex,
      fakeBlockHeaderHex, fakeBlockHeaderHex, fakeBlockHeaderHex]);

    blockHeadersCache.set(1, undefined);
    blockHeadersCache.set(2, fakeBlockHeaderHex);
    blockHeadersCache.set(3, undefined);
    blockHeadersCache.set(4, fakeBlockHeaderHex);
    blockHeadersCache.set(5, fakeBlockHeaderHex);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(blockHash.toString('hex'), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(blockHash
      .toString('hex'), ['height']);
    expect(coreAPIMock.getBlockHeaders).to.be
      .calledOnceWithExactly(blockHash.toString('hex'), 5);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(5);

    expect(cacheSpy.get).to.returned(fakeBlockHeaderHex);
    expect(cacheSpy.get).to.returned(undefined);
  });
});

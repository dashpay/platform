const { BlockHeader } = require('@dashevo/dashcore-lib');
const sinon = require('sinon');
const ChainDataProvider = require('../../../lib/providers/chainDataProvider');
const blockHeadersCache = require('../../../lib/providers/blockheaders-cache');

const headers = [
  {
    version: 2,
    prevHash: '00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c',
    merkleRoot: 'b4fd581bc4bfe51a5a66d8b823bd6ee2b492f0ddc44cf7e820550714cedc117f',
    time: 1398712771,
    bits: '1e0fffff',
    nonce: 31475,
  },
  {
    version: 2,
    prevHash: '0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1',
    merkleRoot: '0d6d332e68eb8ecc66a5baaa95dc4b10c0b32841aed57dc99a5ae0b2f9e4294d',
    time: 1398712772,
    nonce: 6523,
    bits: '1e0ffff0',
  },
  {
    version: 2,
    prevHash: '00000c6264fab4ba2d23990396f42a76aa4822f03cbc7634b79f4dfea36fccc2',
    merkleRoot: '1cc711129405a328c58d1948e748c3b8f3d610e66d9901db88c42c5247829658',
    time: 1398712774,
    nonce: 53194,
    bits: '1e0ffff0',
  },
  {
    version: 2,
    prevHash: '0000057d5c945acbe476bc17bbbaeb2fc1c1b18673e7582c48ac04af61f4d811',
    merkleRoot: '7e6b1b1457308bf6ccb1e325c64607ba7dfac05e26c08887cd28f97d4d4ab3e2',
    time: 1398712782,
    nonce: 193159,
    bits: '1e0ffff0',
  },
  {
    version: 2,
    prevHash: '000002258bd58bf4cdcde282abc030437c103dbb12d2a7dbc978d07bcf386b42',
    merkleRoot: '4cf4f3b788e8dc847a9e0ff3b279340207c555a6bc0736f93e95dbdc2e3c2f16',
    time: 1398712784,
    nonce: 41103,
    bits: '1e0ffff0',
  }];

describe('ChainDataProvider', () => {
  const fakeHeaders = headers.map((e) => new BlockHeader(e));

  let coreAPIMock;
  let chainDataProvider;
  let cacheSpy;

  beforeEach(async function it() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }

    const blockHash = fakeHeaders[0].hash;

    coreAPIMock = {
      getBestChainLock: this.sinon.stub(),
      getBlockHeader: this.sinon.stub(),
      getBlockHeaders: this.sinon.stub(),
      getBlockStats: this.sinon.stub(),
    };
    const zmqClientMock = { on: this.sinon.stub(), topics: { rawblock: '', rawtx: '' } };

    cacheSpy = this.sinon.spy(blockHeadersCache);
    chainDataProvider = new ChainDataProvider(coreAPIMock, zmqClientMock);

    coreAPIMock.getBestChainLock.resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash,
    });

    await chainDataProvider.init();
    coreAPIMock.getBestChainLock.resetHistory();

    blockHeadersCache.purge();

    cacheSpy.set.resetHistory();
    cacheSpy.get.resetHistory();
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  it('should call for chainlock on init', async () => {
    await chainDataProvider.init();

    expect(coreAPIMock.getBestChainLock).to.be.calledOnceWithExactly();
  });

  it('should call for rpc on getBlockHeader when cache is empty', async () => {
    const [fakeBlockHeader] = fakeHeaders;

    coreAPIMock.getBlockHeader.resolves(fakeBlockHeader.toString());

    await chainDataProvider.getBlockHeader(fakeBlockHeader.hash);

    expect(coreAPIMock.getBlockHeader).to.be.calledOnceWithExactly(fakeBlockHeader.hash);

    expect(cacheSpy.get).to.be.calledWith(fakeBlockHeader.hash);
    expect(cacheSpy.get).to.always.returned(undefined);
  });

  it('should return cache on getBlockHeader if it is cached', async () => {
    const [fakeBlockHeader] = fakeHeaders;

    blockHeadersCache.set(fakeBlockHeader.hash, fakeBlockHeader.toString());
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeader(fakeBlockHeader.hash);

    expect(cacheSpy.get.callCount).to.be.equal(1);
    expect(cacheSpy.set.callCount).to.be.equal(0);

    expect(coreAPIMock.getBlockHeader.callCount).to.be.equal(0);
  });

  // the case where we request for N blocks, and theres nothing at all in the cache
  it('should call for rpc when nothing is cached', async () => {
    const [first, second, third, fourth, fifth] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves(fakeHeaders.map((e) => e.toString()));

    await chainDataProvider.getBlockHeaders(first.hash, 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first.hash, ['height']);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(5);

    expect(cacheSpy.get).to.always.returned(undefined);

    expect(cacheSpy.set).to.be.calledWith(1, first.toString());
    expect(cacheSpy.set).to.be.calledWith(2, second.toString());
    expect(cacheSpy.set).to.be.calledWith(3, third.toString());
    expect(cacheSpy.set).to.be.calledWith(4, fourth.toString());
    expect(cacheSpy.set).to.be.calledWith(5, fifth.toString());
  });

  // the case when we request for cached blocks (all N block are cached)
  it('should use cache and do not call for blockHeaders', async () => {
    const [first, second, third] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });

    blockHeadersCache.set(1, first.toString());
    blockHeadersCache.set(2, second.toString());
    blockHeadersCache.set(3, third.toString());

    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(first.hash, 3);

    expect(coreAPIMock.getBlockHeaders.callCount).to.be.equal(0);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first.hash, ['height']);

    expect(cacheSpy.get.callCount).to.be.equal(3);
    expect(cacheSpy.set.callCount).to.be.equal(0);

    expect(cacheSpy.get.getCall(0).returnValue).to.deep.equal(first.toString());
    expect(cacheSpy.get.getCall(1).returnValue).to.deep.equal(second.toString());
    expect(cacheSpy.get.getCall(2).returnValue).to.deep.equal(third.toString());
  });

  // the case where we are missing some blocks in the tail
  // f.e. we request for 5 blocks, and what we have in cache is [1,2,3,undefined,undefined]
  // we should call for 3 blocks (3,4,5) and set 2 missing in the cache
  it('should use cache when miss something in the tail', async () => {
    const [first, second, third, fourth, fifth] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([third.toString(), fourth.toString(),
      fifth.toString()]);

    // should use cache and does not hit rpc
    blockHeadersCache.set(1, first.toString());
    blockHeadersCache.set(2, second.toString());
    blockHeadersCache.set(3, third.toString());
    blockHeadersCache.set(4, undefined);
    blockHeadersCache.set(5, undefined);
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(first.hash, 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first.hash, ['height']);
    expect(coreAPIMock.getBlockHeaders).to.be.calledOnceWithExactly(third.hash, 3);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(3);

    expect(cacheSpy.get.getCall(0).returnValue).to.deep.equal(first.toString());
    expect(cacheSpy.get.getCall(1).returnValue).to.deep.equal(second.toString());
    expect(cacheSpy.get.getCall(2).returnValue).to.deep.equal(third.toString());
    expect(cacheSpy.get.getCall(3).returnValue).to.deep.equal(undefined);
    expect(cacheSpy.get.getCall(4).returnValue).to.deep.equal(undefined);
  });

  // the case when we miss something in the middle
  // f.e we request for 5 blocks, and cache is [1,2,undefined,undefined,5]
  // should take second block as a start point and request for 4 blocks (to the end)
  it('should use cache when missing in the middle', async () => {
    const [first, second, third, fourth, fifth] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([second.toString(), third.toString(),
      fourth.toString(), fifth.toString()]);

    blockHeadersCache.set(1, first.toString());
    blockHeadersCache.set(2, second.toString());
    blockHeadersCache.set(3, undefined);
    blockHeadersCache.set(4, undefined);
    blockHeadersCache.set(5, fifth.toString());
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(first.hash, 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first.hash, ['height']);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(4);

    expect(cacheSpy.get.getCall(0).returnValue).to.deep.equal(first.toString());
    expect(cacheSpy.get.getCall(1).returnValue).to.deep.equal(second.toString());
    expect(cacheSpy.get.getCall(2).returnValue).to.deep.equal(undefined);
    expect(cacheSpy.get.getCall(3).returnValue).to.deep.equal(undefined);
    expect(cacheSpy.get.getCall(4).returnValue).to.deep.equal(fifth.toString());
  });

  // the case where we have something in the cache, but the first blocks are not
  // f.e. we request for 5 blocks, and cache is [undefined,undefined,3,4,5]
  it('should not use cache when miss something in the beginning', async () => {
    const [first,, third, fourth, fifth] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves(fakeHeaders.map((e) => e.toString()));

    blockHeadersCache.set(1, undefined);
    blockHeadersCache.set(2, undefined);
    blockHeadersCache.set(3, third.toString());
    blockHeadersCache.set(4, fourth.toString());
    blockHeadersCache.set(5, fifth.toString());
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(first.toString(), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first.toString(), ['height']);
    expect(coreAPIMock.getBlockHeaders).to.be
      .calledOnceWithExactly(first.toString(), 5);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(5);

    expect(cacheSpy.get.getCall(0).returnValue).to.deep.equal(undefined);
    expect(cacheSpy.get.getCall(1).returnValue).to.deep.equal(undefined);
    expect(cacheSpy.get.getCall(2).returnValue).to.deep.equal(third.toString());
    expect(cacheSpy.get.getCall(3).returnValue).to.deep.equal(fourth.toString());
    expect(cacheSpy.get.getCall(4).returnValue).to.deep.equal(fifth.toString());
  });

  // the same as above, but with additional gap
  // [undefined,2,undefined,4,5]
  it('should not use cache when miss something in the beginning', async () => {
    const [first, second, third, fourth, fifth] = fakeHeaders;

    coreAPIMock.getBlockStats.resolves({ height: 1 });
    coreAPIMock.getBlockHeaders.resolves([first.toString(), second.toString(),
      third.toString(), fourth.toString(), fifth.toString()]);

    blockHeadersCache.set(1, undefined);
    blockHeadersCache.set(2, second.toString());
    blockHeadersCache.set(3, undefined);
    blockHeadersCache.set(4, third.toString());
    blockHeadersCache.set(5, fourth.toString());
    cacheSpy.set.resetHistory();

    await chainDataProvider.getBlockHeaders(first.toString(), 5);

    expect(coreAPIMock.getBlockStats).to.be.calledOnceWithExactly(first
      .toString(), ['height']);
    expect(coreAPIMock.getBlockHeaders).to.be
      .calledOnceWithExactly(first.toString(), 5);

    expect(cacheSpy.get.callCount).to.be.equal(5);
    expect(cacheSpy.set.callCount).to.be.equal(5);
  });
});

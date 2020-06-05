const BloomFilter = require('@dashevo/dashcore-lib/lib/bloomfilter');

const { BloomFilter: BloomFilterMessage } = require('@dashevo/dapi-grpc');

const { EventEmitter } = require('events');
const {
  TransactionsFilterStreamPromiseClient,
  TransactionsWithProofsRequest,
} = require('@dashevo/dapi-grpc');

const subscribeToTransactionsWithProofsFactory = require('../../../../lib/methods/core/subscribeToTransactionsWithProofsFactory');

describe('subscribeToTransactionsWithProofsFactory', () => {
  let subscribeToTransactionsWithProofs;
  let grpcTransportMock;
  let options;
  let stream;

  beforeEach(function beforeEach() {
    options = {
      fromBlockHeight: 1,
      count: 1,
      fromBlockHash: '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2',
    };

    stream = new EventEmitter();
    grpcTransportMock = {
      request: this.sinon.stub().resolves(stream),
    };
    subscribeToTransactionsWithProofs = subscribeToTransactionsWithProofsFactory(grpcTransportMock);
  });

  it('should return a stream', async () => {
    const bloomFilter = BloomFilter.create(1, 0.001);

    const actualStream = await subscribeToTransactionsWithProofs(
      bloomFilter,
      options,
    );

    const bloomFilterMessage = new BloomFilterMessage();

    let { vData } = bloomFilter;

    if (Array.isArray(vData)) {
      vData = new Uint8Array(vData);
    }

    bloomFilterMessage.setVData(vData);
    bloomFilterMessage.setNHashFuncs(bloomFilter.nHashFuncs);
    bloomFilterMessage.setNTweak(bloomFilter.nTweak);
    bloomFilterMessage.setNFlags(bloomFilter.nFlags);

    const request = new TransactionsWithProofsRequest();
    request.setBloomFilter(bloomFilterMessage);
    request.setFromBlockHeight(options.fromBlockHeight);
    request.setCount(options.count);
    request.setFromBlockHash(Buffer.from(options.fromBlockHash, 'hex'));

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      TransactionsFilterStreamPromiseClient,
      'subscribeToTransactionsWithProofs',
      request,
      options,
    );

    expect(actualStream).to.be.equal(stream);
  });
});

import BloomFilter from '@dashevo/dashcore-lib/lib/bloomfilter.js';
import dapiGrpc from '@dashevo/dapi-grpc';
import EventEmitter from 'events';
import subscribeToTransactionsWithProofsFactory from '../../../../lib/methods/core/subscribeToTransactionsWithProofsFactory.js';
import DAPIClientError from '../../../../lib/errors/DAPIClientError.js';
import { hexToBytes } from '../../../../lib/utils/bytes.js';

const {
  v0: {
    BloomFilter: BloomFilterMessage,
    CorePromiseClient,
    TransactionsWithProofsRequest,
  },
} = dapiGrpc;

describe('subscribeToTransactionsWithProofsFactory', () => {
  let subscribeToTransactionsWithProofs;
  let grpcTransportMock;
  let options;
  let stream;

  beforeEach(function beforeEach() {
    options = {
      fromBlockHeight: 1,
      count: 5,
      fromBlockHash: '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2',
      timeout: 150000,
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

    bloomFilterMessage.setVData(new Uint8Array(bloomFilter.vData));
    bloomFilterMessage.setNHashFuncs(bloomFilter.nHashFuncs);
    bloomFilterMessage.setNTweak(bloomFilter.nTweak);
    bloomFilterMessage.setNFlags(bloomFilter.nFlags);

    const request = new TransactionsWithProofsRequest();
    request.setBloomFilter(bloomFilterMessage);
    request.setFromBlockHeight(options.fromBlockHeight);
    request.setCount(options.count);
    request.setFromBlockHash(hexToBytes(options.fromBlockHash));

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'subscribeToTransactionsWithProofs',
      request,
      options,
    );

    expect(actualStream).to.be.equal(stream);
  });

  it('should apply default options', async () => {
    const bloomFilter = BloomFilter.create(1, 0.001);

    const actualStream = await subscribeToTransactionsWithProofs(
      bloomFilter,
    );

    const bloomFilterMessage = new BloomFilterMessage();

    bloomFilterMessage.setVData(new Uint8Array(bloomFilter.vData));
    bloomFilterMessage.setNHashFuncs(bloomFilter.nHashFuncs);
    bloomFilterMessage.setNTweak(bloomFilter.nTweak);
    bloomFilterMessage.setNFlags(bloomFilter.nFlags);

    const request = new TransactionsWithProofsRequest();
    request.setBloomFilter(bloomFilterMessage);
    request.setCount(0);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'subscribeToTransactionsWithProofs',
      request,
      {
        count: 0,
        timeout: undefined,
      },
    );

    expect(actualStream).to.be.equal(stream);
  });

  it('should throw error if `fromBlockHeight` is set to 0', async () => {
    options = {
      fromBlockHeight: 0,
      count: 5,
      fromBlockHash: '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2',
      timeout: 150000,
    };

    const bloomFilter = BloomFilter.create(1, 0.001);

    try {
      await subscribeToTransactionsWithProofs(bloomFilter, options);
    } catch (e) {
      expect(e).to.be.an.instanceOf(DAPIClientError);
      expect(e.message).to.equal('Invalid argument: minimum value for `fromBlockHeight` is 1');
    }
  });
});

import dapiGrpc from '@dashevo/dapi-grpc';
import EventEmitter from 'events';
import subscribeToBlockHeadersWithChainLocksFactory from '../../../../lib/methods/core/subscribeToBlockHeadersWithChainLocksFactory.js';
import DAPIClientError from '../../../../lib/errors/DAPIClientError.js';
import { hexToBytes } from '../../../../lib/utils/bytes.js';

const {
  v0: {
    CorePromiseClient,
    BlockHeadersWithChainLocksRequest,
  },
} = dapiGrpc;

describe('subscribeToBlockHeadersWithChainLocks', () => {
  let subscribeToBlockHeadersWithChainLocks;
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

    // eslint-disable-next-line operator-linebreak
    subscribeToBlockHeadersWithChainLocks =
      subscribeToBlockHeadersWithChainLocksFactory(grpcTransportMock);
  });

  it('should return a stream', async () => {
    try {
      await subscribeToBlockHeadersWithChainLocks(
        { ...options, fromBlockHeight: 0 },
      );
    } catch (e) {
      expect(e).to.be.an.instanceOf(DAPIClientError);
    }

    const actualStream = await subscribeToBlockHeadersWithChainLocks(
      { ...options, fromBlockHeight: 1 },
    );

    const request = new BlockHeadersWithChainLocksRequest();

    request.setFromBlockHeight(1);
    request.setFromBlockHash(hexToBytes(options.fromBlockHash));
    request.setCount(options.count);

    expect(grpcTransportMock.request).to.be.calledWith(
      CorePromiseClient,
      'subscribeToBlockHeadersWithChainLocks',
      request,
      options,
    );

    expect(actualStream).to.be.equal(stream);
  });
});

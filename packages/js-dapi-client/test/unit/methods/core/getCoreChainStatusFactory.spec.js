const {
  v0: {
    GetCoreChainStatusRequest,
    GetCoreChainStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getCoreChainStatusFactory = require('../../../../lib/methods/core/getCoreChainStatusFactory');

describe('getCoreChainStatusFactory', () => {
  let getCoreChainStatus;
  let grpcTransportMock;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getCoreChainStatus = getCoreChainStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetCoreChainStatusResponse();

    response.setStatus(GetCoreChainStatusResponse.Status.READY);

    const chain = new GetCoreChainStatusResponse.Chain();
    chain.setBestBlockHash(Buffer.from('bestBlockHash'));

    response.setChain(chain);

    grpcTransportMock.request.resolves(response);

    const options = {
      timeout: 1000,
    };

    const result = await getCoreChainStatus(
      options,
    );

    const request = new GetCoreChainStatusRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getCoreChainStatus',
      request,
      options,
    );

    const expectedResult = {
      ...response.toObject(),
      status: 'READY',
    };

    expectedResult.chain.bestBlockHash = Buffer.from(expectedResult.chain.bestBlockHash, 'base64');

    expect(result).to.deep.equal(expectedResult);
  });
});

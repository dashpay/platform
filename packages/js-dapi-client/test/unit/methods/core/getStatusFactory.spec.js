const {
  v0: {
    GetStatusRequest,
    GetStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getStatusFactory = require('../../../../lib/methods/core/getStatusFactory');

describe('getStatusFactory', () => {
  let getStatus;
  let grpcTransportMock;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getStatus = getStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetStatusResponse();

    response.setStatus(GetStatusResponse.Status.READY);

    const masternode = new GetStatusResponse.Masternode();

    masternode.setStatus(GetStatusResponse.Masternode.Status.READY);

    const chain = new GetStatusResponse.Chain();
    chain.setBestBlockHash(Buffer.from('bestBlockHash'));

    response.setMasternode(masternode);
    response.setChain(chain);

    grpcTransportMock.request.resolves(response);

    const options = {
      timeout: 1000,
    };

    const result = await getStatus(
      options,
    );

    const request = new GetStatusRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getStatus',
      request,
      options,
    );

    const expectedResult = {
      ...response.toObject(),
      status: 'READY',
      masternode: {
        ...response.getMasternode().toObject(),
        status: 'READY',
      },
    };

    expectedResult.chain.bestBlockHash = Buffer.from(expectedResult.chain.bestBlockHash, 'base64');

    expect(result).to.deep.equal(expectedResult);
  });
});

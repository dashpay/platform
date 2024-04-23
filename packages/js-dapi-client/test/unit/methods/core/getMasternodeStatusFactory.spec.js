const {
  v0: {
    GetMasternodeStatusRequest,
    GetMasternodeStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const getMasternodeStatusFactory = require('../../../../lib/methods/core/getMasternodeStatusFactory');

describe('getMasternodeStatusFactory', () => {
  let getMasternodeStatus;
  let grpcTransportMock;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };
    getMasternodeStatus = getMasternodeStatusFactory(grpcTransportMock);
  });

  it('should return status', async () => {
    const response = new GetMasternodeStatusResponse();

    response.setStatus(GetMasternodeStatusResponse.Status.READY);

    grpcTransportMock.request.resolves(response);

    const options = {
      timeout: 1000,
    };

    const result = await getMasternodeStatus(
      options,
    );

    const request = new GetMasternodeStatusRequest();

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getMasternodeStatus',
      request,
      options,
    );

    const expectedResult = {
      ...response.toObject(),
      proTxHash: Buffer.alloc(0),
      status: 'READY',
    };

    expect(result).to.deep.equal(expectedResult);
  });
});

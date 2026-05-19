import dapiGrpc from '@dashevo/dapi-grpc';
import getMasternodeStatusFactory from '../../../../lib/methods/core/getMasternodeStatusFactory.js';

const {
  v0: {
    GetMasternodeStatusRequest,
    GetMasternodeStatusResponse,
    CorePromiseClient,
  },
} = dapiGrpc;

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
      proTxHash: new Uint8Array(0),
      status: 'READY',
    };

    expect(result).to.deep.equal(expectedResult);
  });
});

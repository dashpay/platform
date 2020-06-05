const {
  ApplyStateTransitionRequest,
  ApplyStateTransitionResponse,
  PlatformPromiseClient,
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const broadcastStateTransitionFactory = require('../../../../lib/methods/platform/broadcastStateTransitionFactory');

describe('broadcastStateTransitionFactory', () => {
  let grpcTransportMock;
  let broadcastStateTransition;
  let options;
  let stateTransitionFixture;
  let response;

  beforeEach(function beforeEach() {
    response = new ApplyStateTransitionResponse();

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    const dataContractFixture = getDataContractFixture();
    const dpp = new DashPlatformProtocol();
    stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);

    options = {
      timeout: 1000,
    };

    broadcastStateTransition = broadcastStateTransitionFactory(grpcTransportMock);
  });

  it('should broadcast state transition', async () => {
    const result = await broadcastStateTransition(stateTransitionFixture, options);

    const request = new ApplyStateTransitionRequest();
    request.setStateTransition(stateTransitionFixture);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'applyStateTransition',
      request,
      options,
    );
    expect(result).to.equal(response);
  });
});

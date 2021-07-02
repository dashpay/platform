const {
  v0: {
    BroadcastStateTransitionRequest,
    PlatformPromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const broadcastStateTransitionFactory = require('../../../../../lib/methods/platform/broadcastStateTransition/broadcastStateTransitionFactory');
const BroadcastStateTransitionResponse = require('../../../../../lib/methods/platform/broadcastStateTransition/BroadcastStateTransitionResponse');

describe('broadcastStateTransitionFactory', () => {
  let grpcTransportMock;
  let broadcastStateTransition;
  let options;
  let stateTransitionFixture;
  let response;

  beforeEach(async function beforeEach() {
    response = new BroadcastStateTransitionResponse();

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    const dataContractFixture = getDataContractFixture();
    const dpp = new DashPlatformProtocol();
    await dpp.initialize();

    stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);

    options = {
      timeout: 1000,
    };

    broadcastStateTransition = broadcastStateTransitionFactory(grpcTransportMock);
  });

  it('should broadcast state transition', async () => {
    const result = await broadcastStateTransition(stateTransitionFixture, options);

    const request = new BroadcastStateTransitionRequest();
    request.setStateTransition(stateTransitionFixture);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'broadcastStateTransition',
      request,
      options,
    );
    expect(result).to.be.an.instanceOf(BroadcastStateTransitionResponse);
  });
});

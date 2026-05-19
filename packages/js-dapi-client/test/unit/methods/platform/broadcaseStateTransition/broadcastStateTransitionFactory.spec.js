import dapiGrpc from '@dashevo/dapi-grpc';
import wasmDpp from '@dashevo/wasm-dpp';
import getDataContractFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture.js';
import broadcastStateTransitionFactory from '../../../../../lib/methods/platform/broadcastStateTransition/broadcastStateTransitionFactory.js';
import BroadcastStateTransitionResponse from '../../../../../lib/methods/platform/broadcastStateTransition/BroadcastStateTransitionResponse.js';

const {
  v0: {
    BroadcastStateTransitionRequest,
    PlatformPromiseClient,
  },
} = dapiGrpc;

const { DashPlatformProtocol } = wasmDpp;

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

    const dataContractFixture = await getDataContractFixture();
    const dpp = new DashPlatformProtocol(null, 1);

    stateTransitionFixture = dpp.dataContract.createDataContractCreateTransition(
      dataContractFixture,
    );

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

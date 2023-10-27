const {
  v0: {
    ResponseMetadata,
    GetDataContractResponse,
    GetDocumentsResponse,
    GetIdentityResponse,
    BroadcastStateTransitionResponse,
    WaitForStateTransitionResultResponse,
  },
} = require('@dashevo/dapi-grpc');

const { DashPlatformProtocol } = require('@dashevo/wasm-dpp');

const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const PlatformMethodsFacade = require('../../../../lib/methods/platform/PlatformMethodsFacade');

const { WaitForStateTransitionResultResponseV0 } = WaitForStateTransitionResultResponse;
const { GetIdentityResponseV0 } = GetIdentityResponse;
const { GetDocumentsResponseV0 } = GetDocumentsResponse;
const { GetDataContractResponseV0 } = GetDataContractResponse;

describe('PlatformMethodsFacade', () => {
  let grpcTransportMock;
  let platformMethods;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };

    platformMethods = new PlatformMethodsFacade(grpcTransportMock);
  });

  describe('#broadcastStateTransition', () => {
    it('should broadcast state transition', async () => {
      const response = new BroadcastStateTransitionResponse();
      grpcTransportMock.request.resolves(response);

      const dpp = new DashPlatformProtocol(null, 1);
      const stateTransition = dpp.dataContract.createDataContractCreateTransition(
        await getDataContractFixture(),
      );

      await platformMethods.broadcastStateTransition(stateTransition);

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getDataContract', () => {
    it('should get data contract', async () => {
      const response = new GetDataContractResponse();
      response.setV0(
        new GetDataContractResponseV0()
          .setMetadata(new ResponseMetadata())
          .setDataContract((await getDataContractFixture()).toBuffer()),
      );
      grpcTransportMock.request.resolves(response);

      await platformMethods.getDataContract((await getDataContractFixture()).getId());

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getDocuments', () => {
    it('should get documents', async () => {
      const response = new GetDocumentsResponse();
      response.setV0(
        new GetDocumentsResponseV0()
          .setMetadata(new ResponseMetadata()),
      );
      grpcTransportMock.request.resolves(response);

      await platformMethods.getDocuments(
        '11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c',
        'niceDocument',
      );

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getIdentity', () => {
    it('should get Identity', async () => {
      const response = new GetIdentityResponse();
      response.setV0(
        new GetIdentityResponseV0()
          .setMetadata(new ResponseMetadata())
          .setIdentity((await getIdentityFixture()).toBuffer()),
      );

      grpcTransportMock.request.resolves(response);

      await platformMethods.getIdentity('41nthkqvHBLnqiMkSbsdTNANzYu9bgdv4etKoRUunY1M');

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#waitForStateTransitionResult', () => {
    it('should wait for state transition', async () => {
      const response = new WaitForStateTransitionResultResponse();
      response.setV0(
        new WaitForStateTransitionResultResponseV0()
          .setMetadata(new ResponseMetadata()),
      );

      grpcTransportMock.request.resolves(response);

      await platformMethods.waitForStateTransitionResult(
        Buffer.from('6f49655a2906852a38e473dd47574fb70b8b7c4e5cee9ea8e7da3f07b970c421', 'hex'),
        false,
      );

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });
});

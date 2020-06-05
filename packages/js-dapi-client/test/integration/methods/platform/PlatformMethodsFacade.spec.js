const {
  GetDataContractResponse,
  GetDocumentsResponse,
  GetIdentityByFirstPublicKeyResponse,
  GetIdentityResponse,
  GetIdentityIdByFirstPublicKeyResponse,
  ApplyStateTransitionResponse,
} = require('@dashevo/dapi-grpc');
const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const PlatformMethodsFacade = require('../../../../lib/methods/platform/PlatformMethodsFacade');

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
      const response = new ApplyStateTransitionResponse();
      grpcTransportMock.request.resolves(response);

      const dpp = new DashPlatformProtocol();
      const stateTransition = dpp.dataContract.createStateTransition(getDataContractFixture());

      await platformMethods.broadcastStateTransition(stateTransition);

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getDataContract', () => {
    it('should get data contract', async () => {
      const response = new GetDataContractResponse();
      grpcTransportMock.request.resolves(response);

      await platformMethods.getDataContract(getDataContractFixture().getId());

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getDocuments', () => {
    it('should get documents', async () => {
      const response = new GetDocumentsResponse();
      grpcTransportMock.request.resolves(response);

      await platformMethods.getDocuments(
        '11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c',
        'niceDocument',
      );

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getIdentityByFirstPublicKey', () => {
    it('should get Identity by first public key', async () => {
      const response = new GetIdentityByFirstPublicKeyResponse();
      grpcTransportMock.request.resolves(response);

      await platformMethods.getIdentityByFirstPublicKey('556c2910d46fda2b327ef9d9bda850cc84d30db0');

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getIdentity', () => {
    it('should get Identity', async () => {
      const response = new GetIdentityResponse();
      grpcTransportMock.request.resolves(response);

      await platformMethods.getIdentity('41nthkqvHBLnqiMkSbsdTNANzYu9bgdv4etKoRUunY1M');

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getIdentityIdByFirstPublicKey', () => {
    it('should get Identity ID by first public key', async () => {
      const response = new GetIdentityIdByFirstPublicKeyResponse();
      grpcTransportMock.request.resolves(response);

      await platformMethods.getIdentityIdByFirstPublicKey('556c2910d46fda2b327ef9d9bda850cc84d30db0');

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });
});

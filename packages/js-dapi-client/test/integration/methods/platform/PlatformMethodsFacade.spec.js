const {
  v0: {
    ResponseMetadata,
    GetDataContractResponse,
    GetDocumentsResponse,
    GetIdentityResponse,
    GetEpochsInfoResponse,
    GetVersionUpgradeVoteStatusResponse,
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
const { GetEpochsInfoResponseV0 } = GetEpochsInfoResponse;
const { GetVersionUpgradeVoteStatusResponseV0 } = GetVersionUpgradeVoteStatusResponse;

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

  describe('#getEpochsInfo', () => {
    it('should get epochs info', async () => {
      const { EpochInfo, EpochInfos } = GetEpochsInfoResponseV0;

      const response = new GetEpochsInfoResponse();

      response.setV0(
        new GetEpochsInfoResponseV0()
          .setEpochs(new EpochInfos()
            .setEpochInfosList([new EpochInfo()
              .setNumber(1)
              .setFirstBlockHeight(1)
              .setFirstCoreBlockHeight(1)
              .setStartTime(Date.now())
              .setFeeMultiplier(1)]))
          .setMetadata(new ResponseMetadata()),
      );

      grpcTransportMock.request.resolves(response);

      await platformMethods.getEpochsInfo(1, 1, {});

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getVersionUpgradeVoteStatus', () => {
    it('should get version upgrade vote status', async () => {
      const startProTxHash = Buffer.alloc(32).fill('a').toString('hex');
      const proTxHash = Buffer.alloc(32).fill('b').toString('hex');

      const { VersionSignal, VersionSignals } = GetVersionUpgradeVoteStatusResponseV0;

      const response = new GetVersionUpgradeVoteStatusResponse();

      response.setV0(
        new GetVersionUpgradeVoteStatusResponseV0()
          .setVersions(new VersionSignals()
            .setVersionSignalsList([new VersionSignal()
              .setProTxHash(proTxHash)
              .setVersion(1)]))
          .setMetadata(new ResponseMetadata()),
      );

      grpcTransportMock.request.resolves(response);

      await platformMethods.getVersionUpgradeVoteStatus(startProTxHash, 1, {});

      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });
});

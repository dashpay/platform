import getConfigMock from '../../../../src/test/mock/getConfigMock.js';
import getMasternodeScopeFactory from '../../../../src/status/scopes/masternode.js';
import { MasternodeSyncAssetEnum } from '../../../../src/status/enums/masternodeSyncAsset.js';
import { MasternodeStateEnum } from '../../../../src/status/enums/masternodeState.js';

describe('getMasternodeScopeFactory', () => {
  describe('#getMasternodeScope', () => {
    let mockRpcClient;
    let mockCreateRpcClient;
    let mockDockerCompose;
    let mockGetConnectionHost;

    let config;
    let getMasternodeScope;

    beforeEach(async function it() {
      mockRpcClient = {
        mnsync: this.sinon.stub(),
        getBlockchainInfo: this.sinon.stub(),
        masternode: this.sinon.stub(),
      };
      mockCreateRpcClient = () => mockRpcClient;
      mockDockerCompose = { execCommand: this.sinon.stub() };
      mockGetConnectionHost = this.sinon.stub();

      config = getConfigMock(this.sinon);
      getMasternodeScope = getMasternodeScopeFactory(
        mockDockerCompose,
        mockCreateRpcClient,
        mockGetConnectionHost,
      );
    });

    it('should just work', async () => {
      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        },
      });

      const mockProTxHash = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
      const mockDmnState = {
        PoSePenalty: 0,
        PoSeRevivedHeight: 500,
        lastPaidHeight: 555,
        registeredHeight: 400,
      };

      mockRpcClient.getBlockchainInfo.returns({ result: { blocks: 1337 } });
      mockRpcClient.masternode.withArgs('count').returns({
        result: {
          detailed: { regular: { total: 1337, enabled: 777 }, evo: { total: 1337, enabled: 777 } },
        },
      });
      mockRpcClient.masternode.withArgs('status').returns({
        result: {
          dmnState: mockDmnState,
          state: MasternodeStateEnum.READY,
          status: 'Ready',
          proTxHash: mockProTxHash,
        },
      });

      const scope = await getMasternodeScope(config);

      const expectedScope = {
        syncAsset: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        proTxHash: mockProTxHash,
        state: MasternodeStateEnum.READY,
        status: 'Ready',
        masternodeTotal: 1337,
        masternodeEnabled: 777,
        evonodeTotal: 1337,
        evonodeEnabled: 777,
        nodeState: {
          dmnState: mockDmnState,
          poSePenalty: mockDmnState.PoSePenalty,
          lastPaidHeight: mockDmnState.lastPaidHeight,
          // ignore these 3
          lastPaidTime: scope.nodeState.lastPaidTime,
          nextPaymentTime: scope.nodeState.nextPaymentTime,
          paymentQueuePosition: scope.nodeState.paymentQueuePosition,
        },
      };

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should set mnsync null', async () => {
      // simulate failed request to dashcore
      mockRpcClient.mnsync.throws(new Error());

      // it should not be trying to obtain masternode info
      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
    });

    it('should not request masternode info if syncing', async () => {
      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN,
        },
      });

      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
      expect(mockRpcClient.masternode.notCalled).to.be.true();
    });
  });
});

const MasternodeSyncAssetEnum = require('../../../../src/enums/masternodeSyncAsset');
const getMasternodeScopeFactory = require('../../../../src/status/scopes/masternode');
const MasternodeStateEnum = require('../../../../src/enums/masternodeState');

describe('getMasternodeScopeFactory', () => {
  describe('#getMasternodeScope', () => {
    let mockRpcClient;
    let mockCreateRpcClient;
    let mockDockerCompose;

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

      config = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };
      getMasternodeScope = getMasternodeScopeFactory(mockDockerCompose, mockCreateRpcClient);
    });

    it('should just work', async () => {
      config.toEnvs.returns({});

      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        },
      });

      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py')
        .returns({ out: '' });

      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v')
        .returns({ out: 'Dash Sentinel v1.7.1' });

      const mockProTxHash = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
      const mockDmnState = {
        PoSePenalty: 0,
        PoSeRevivedHeight: 500,
        lastPaidHeight: 555,
        registeredHeight: 400,
      };

      mockRpcClient.getBlockchainInfo.returns({ result: { blocks: 1337 } });
      mockRpcClient.masternode.withArgs('count').returns({ result: { enabled: 666 } });
      mockRpcClient.masternode.withArgs('status').returns({
        result: {
          dmnState: mockDmnState,
          state: MasternodeStateEnum.READY,
          status: 'Ready',
          proTxHash: mockProTxHash,
        },
      });

      const scope = await getMasternodeScope(config);

      expect(scope.syncAsset).to.be.equal(MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED);

      expect(scope.proTxHash).to.be.equal(mockProTxHash);
      expect(scope.state).to.be.equal(MasternodeStateEnum.READY);
      expect(scope.status).to.be.equal('Ready');

      expect(scope.sentinel.state).to.be.equal('ok');
      expect(scope.sentinel.version).to.be.equal('1.7.1');
      expect(scope.nodeState.dmnState).to.be.equal(mockDmnState);
      expect(scope.nodeState.poSePenalty).to.be.equal(mockDmnState.PoSePenalty);
      expect(scope.nodeState.lastPaidHeight).to.be.equal(mockDmnState.lastPaidHeight);
      expect(scope.nodeState.lastPaidTime).to.exist();
      expect(scope.nodeState.paymentQueuePosition).to.exist();
      expect(scope.nodeState.nextPaymentTime).to.exist();
    });

    it('should throw if error with calls to core', async () => {
      mockRpcClient.mnsync.throws(new Error());

      try {
        await getMasternodeScope(config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e instanceof Error).to.be.true();
      }

      expect(mockDockerCompose.execCommand.notCalled).to.be.true();
      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
      expect(mockRpcClient.masternode.notCalled).to.be.true();
    });

    it('should not make calls if syncing', async () => {
      config.toEnvs.returns({});

      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN,
        },
      });

      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py')
        .returns({ out: 'Waiting for dash core sync' });

      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v')
        .returns({ out: 'Dash Sentinel v1.7.1' });

      const scope = await getMasternodeScope(config);

      expect(scope.syncAsset).to.be.equal(MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN);

      expect(scope.proTxHash).to.be.equal(null);
      expect(scope.state).to.be.equal(null);
      expect(scope.status).to.be.equal(null);

      expect(scope.nodeState.poSePenalty).to.be.equal(null);
      expect(scope.nodeState.lastPaidHeight).to.be.equal(null);
      expect(scope.nodeState.lastPaidTime).to.be.equal(null);
      expect(scope.nodeState.paymentQueuePosition).to.be.equal(null);
      expect(scope.nodeState.nextPaymentTime).to.be.equal(null);

      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
      expect(mockRpcClient.masternode.notCalled).to.be.true();
    });

    it('should not load sentinel version if exited with error', async () => {
      config.toEnvs.returns({});

      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN,
        },
      });

      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py')
        .throws(new Error());

      const mockProTxHash = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
      const mockDmnState = {
        PoSePenalty: 0,
        PoSeRevivedHeight: 500,
        lastPaidHeight: 555,
        registeredHeight: 400,
      };

      mockRpcClient.getBlockchainInfo.returns({ result: { blocks: 1337 } });
      mockRpcClient.masternode.withArgs('count').returns({ result: { enabled: 666 } });
      mockRpcClient.masternode.withArgs('status').returns({
        result: {
          dmnState: mockDmnState,
          state: MasternodeStateEnum.READY,
          status: 'Ready',
          proTxHash: mockProTxHash,
        },
      });

      const scope = await getMasternodeScope(config);

      expect(scope.sentinel.state).to.be.equal(null);
      expect(scope.sentinel.version).to.be.equal(null);
    });
  });
});

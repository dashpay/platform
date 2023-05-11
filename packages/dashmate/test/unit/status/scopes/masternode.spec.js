const MasternodeSyncAssetEnum = require('../../../../src/status/enums/masternodeSyncAsset');
const getMasternodeScopeFactory = require('../../../../src/status/scopes/masternode');
const MasternodeStateEnum = require('../../../../src/status/enums/masternodeState');

xdescribe('getMasternodeScopeFactory', () => {
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

      config = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };
      getMasternodeScope = getMasternodeScopeFactory(mockDockerCompose,
        mockCreateRpcClient, mockGetConnectionHost);
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
        .returns({ out: 'Dash Sentinel v1.7.3' });

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
      expect(scope.sentinel.version).to.be.equal('1.7.3');
      expect(scope.nodeState.dmnState).to.be.equal(mockDmnState);
      expect(scope.nodeState.poSePenalty).to.be.equal(mockDmnState.PoSePenalty);
      expect(scope.nodeState.lastPaidHeight).to.be.equal(mockDmnState.lastPaidHeight);
      expect(scope.nodeState.lastPaidTime).to.exist();
      expect(scope.nodeState.paymentQueuePosition).to.exist();
      expect(scope.nodeState.nextPaymentTime).to.exist();
    });


    it('should set mnsync null', async () => {
      // simulate failed request to dashcore
      mockRpcClient.mnsync.throws(new Error())

      // and lets say sentinel is working
      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py')
        .returns({ out: 'Waiting for dash core sync' });
      mockDockerCompose.execCommand
        .withArgs(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v')
        .returns({ out: 'Dash Sentinel v1.7.3' });

      const scope = await getMasternodeScope(config)


    // should return scope with no info, but sentinel is in there
      const expectedScope = {
        syncAsset: null,
        sentinel: {
          state: 'ok',
          version: '1.7.3',
        },
        proTxHash: null,
        state: MasternodeStateEnum.UNKNOWN,
        status: null,
        nodeState: {
          dmnState: null,
          poSePenalty: null,
          lastPaidHeight: null,
          lastPaidTime: null,
          paymentQueuePosition: null,
          nextPaymentTime: null,
        },
      };

      expect(scope).to.be.deepEqual(expectedScope)

      // and also should not be trying to obtain masternode info
      expect(mockDockerCompose.execCommand.notCalled).to.be.true();
      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
    });

    it('should not request masternode info if syncing', async () => {
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
        .returns({ out: 'Dash Sentinel v1.7.3' });

      const scope = await getMasternodeScope(config);

      expect(scope.syncAsset).to.be.equal(MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN);

      expect(scope.proTxHash).to.be.equal(null);
      expect(scope.state).to.be.equal(MasternodeStateEnum.UNKNOWN);
      expect(scope.status).to.be.equal(null);

      expect(scope.nodeState.poSePenalty).to.be.equal(null);
      expect(scope.nodeState.lastPaidHeight).to.be.equal(null);
      expect(scope.nodeState.lastPaidTime).to.be.equal(null);
      expect(scope.nodeState.paymentQueuePosition).to.be.equal(null);
      expect(scope.nodeState.nextPaymentTime).to.be.equal(null);

      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();
      expect(mockRpcClient.masternode.notCalled).to.be.true();
    });
  });
});

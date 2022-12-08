// const sinon = require('sinon');
// const MasternodeSyncAssetEnum = require('../../../src/enums/masternodeSyncAsset');
// const getOverviewScopeFactory = require('../../../src/status/scopes/overview');
//
// describe('determineStatus', () => {
//   let mockRpcClient;
//   let mockCreateRpcClient;
//   let mockDockerCompose;
//
//   beforeEach(async () => {
//     mockRpcClient = {
//       mnsync: sinon.stub(),
//       getBlockchainInfo: sinon.stub(),
//       getNetworkInfo: sinon.stub(),
//     };
//     mockCreateRpcClient = () => mockRpcClient;
//     mockDockerCompose = { inspectService: sinon.stub() };
//   });
//
//   it('should not retrieve masternode and platform data', async () => {
//     const config = { get: sinon.stub(), toEnvs: sinon.stub() };
//
//     mockRpcClient.mnsync.returns({
//       result: {
//         AssetName:
//         MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
//       },
//     });
//     mockRpcClient.getBlockchainInfo.returns({
//       result: {
//         size_on_disk: 1337,
//         verificationprogress: 1,
//       },
//     });
//     mockRpcClient.getNetworkInfo.returns({ result: { subversion: '' } });
//     mockDockerCompose.inspectService.returns({ State: { Status: 'running' } });
//     config.get.withArgs('network').returns('mainnet');
//
//     const mockGetPlatformScope = sinon.stub();
//     const mockGetMasternodeScope = sinon.stub();
//
//     const getOverviewScope = getOverviewScopeFactory(mockDockerCompose, mockCreateRpcClient);
//
//     const scope = await getOverviewScope(config, mockGetPlatformScope, mockGetMasternodeScope);
//
//     expect(mockGetMasternodeScope.notCalled).to.be.true();
//     expect(mockGetPlatformScope.notCalled).to.be.true();
//   });
// });

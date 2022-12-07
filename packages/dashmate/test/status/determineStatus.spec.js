const sinon = require('sinon');
const MasternodeSyncAssetEnum = require('../../src/enums/masternodeSyncAsset');
const ServiceStatusEnum = require('../../src/enums/serviceStatus');
const DockerStatusEnum = require('../../src/enums/dockerStatus');
const determineStatus = require('../../src/status/determineStatus');

describe('determineStatus', () => {
  let dockerComposeMock;

  const config = { toEnvs: sinon.stub() };

  beforeEach(async () => {
    dockerComposeMock = { inspectService: sinon.stub() };
  });

  it('should return status from Docker', async () => {
    const mockDockerStatus = 'running';

    dockerComposeMock.inspectService.resolves({ State: { Status: mockDockerStatus } });

    const status = await determineStatus.docker(dockerComposeMock, config, 'sample_service');

    expect(status).to.equal(mockDockerStatus);
  });

  it('should return syncing', async () => {
    const syncing = determineStatus.core(DockerStatusEnum.running, MasternodeSyncAssetEnum.MASTERNODE_SYNC_INITIAL);
    const up = determineStatus.core(DockerStatusEnum.running, MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED);
    const error = determineStatus.core(DockerStatusEnum.restarting, MasternodeSyncAssetEnum.MASTERNODE_SYNC_INITIAL);

    expect(syncing).to.be.equal(ServiceStatusEnum.syncing);
    expect(up).to.be.equal(ServiceStatusEnum.up);
    expect(error).to.be.equal(ServiceStatusEnum.error);
  });
});

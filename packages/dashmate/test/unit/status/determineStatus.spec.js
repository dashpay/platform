import { MasternodeSyncAssetEnum } from '../../../src/status/enums/masternodeSyncAsset.js';
import { ServiceStatusEnum } from '../../../src/status/enums/serviceStatus.js';
import { DockerStatusEnum } from '../../../src/status/enums/dockerStatus.js';
import determineStatus from '../../../src/status/determineStatus.js';
import getConfigMock from '../../../src/test/mock/getConfigMock.js';

describe('determineStatus', () => {
  describe('#docker', () => {
    let dockerComposeMock;
    let config;

    beforeEach(async function it() {
      config = getConfigMock(this.sinon);
      dockerComposeMock = { inspectService: this.sinon.stub() };
    });

    it('should return status from Docker', async () => {
      const mockDockerStatus = 'running';

      dockerComposeMock.inspectService.resolves({ State: { Status: mockDockerStatus } });

      const status = await determineStatus.docker(dockerComposeMock, config, 'sample_service');

      expect(status).to.equal(mockDockerStatus);
    });

    describe('#core', () => {
      it('should return up', async () => {
        const dockerStatus = DockerStatusEnum.running;
        const syncAsset = MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED;
        expect(determineStatus.core(dockerStatus, syncAsset)).to.equal(ServiceStatusEnum.up);
      });

      it('should return syncing', async () => {
        expect(determineStatus.core(
          DockerStatusEnum.running,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_INITIAL,
        )).to.equal(ServiceStatusEnum.syncing);
        expect(determineStatus.core(
          DockerStatusEnum.running,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN,
        )).to.equal(ServiceStatusEnum.syncing);
        expect(determineStatus.core(
          DockerStatusEnum.running,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_GOVERNANCE,
        )).to.equal(ServiceStatusEnum.syncing);
      });

      it('should return error', async () => {
        expect(determineStatus.core(DockerStatusEnum.running, null))
          .to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.restarting,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.created,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.dead,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.created,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.restarting,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.exited,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
        expect(determineStatus.core(
          DockerStatusEnum.removing,
          MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        )).to.equal(ServiceStatusEnum.error);
      });

      it('should return error', async () => {
        const mockDockerStatus = 'running';

        dockerComposeMock.inspectService.resolves({ State: { Status: mockDockerStatus } });

        const status = await determineStatus.docker(dockerComposeMock, config, 'sample_service');

        expect(status).to.equal(mockDockerStatus);
      });
    });
  });

  describe('#platform', () => {
    it('should return syncing', async () => {
      const syncing = determineStatus.core(
        DockerStatusEnum.running,
        MasternodeSyncAssetEnum.MASTERNODE_SYNC_INITIAL,
      );
      const up = determineStatus.core(
        DockerStatusEnum.running,
        MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
      );
      const error = determineStatus.core(
        DockerStatusEnum.restarting,
        MasternodeSyncAssetEnum.MASTERNODE_SYNC_INITIAL,
      );

      expect(syncing).to.be.equal(ServiceStatusEnum.syncing);
      expect(up).to.be.equal(ServiceStatusEnum.up);
      expect(error).to.be.equal(ServiceStatusEnum.error);
    });
  });
});

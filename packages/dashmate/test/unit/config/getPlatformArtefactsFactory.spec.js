import fs from 'fs';
import path from 'path';
import HomeDir from '../../../src/config/HomeDir.js';
import Config from '../../../src/config/Config.js';
import getPlatformArtefactsFactory from '../../../src/config/getPlatformArtefactsFactory.js';

describe('getPlatformArtefacts', () => {
  let homeDir;
  let config;
  let dockerCompose;
  let getPlatformArtefacts;

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();
    config = new Config('testnet', {}, true);

    dockerCompose = {
      throwErrorIfNotInstalled: this.sinon.stub().resolves(),
      getVolumeCandidates: this.sinon.stub().resolves([]),
      // Docker has it, and Compose created it for the project
      isVolumeOwned: this.sinon.stub().resolves(true),
      getContainersList: this.sinon.stub().resolves([]),
    };

    getPlatformArtefacts = getPlatformArtefactsFactory(
      homeDir,
      dockerCompose,
      () => [
        { name: 'core', profiles: ['core'] },
        { name: 'drive_abci', profiles: ['platform'] },
        { name: 'quorum_list', profiles: ['platform-quorum'] },
      ],
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  it('should find nothing on a node a hard reset just cleaned', async () => {
    expect(await getPlatformArtefacts(config)).to.deep.equal({
      certificateFiles: [],
      dataVolumes: [],
      activeServices: [],
    });
  });

  // The quorum list is in a profile of its own, enabled or not, and its
  // volumes and containers belong to Platform as much as Drive's.
  it('should look at every Platform service, with every profile enabled', async () => {
    await getPlatformArtefacts(config);

    // The volumes the reset removes: it removes the helper too.
    expect(dockerCompose.getVolumeCandidates).to.have.been.calledOnceWithExactly(config, {
      serviceNames: ['drive_abci', 'quorum_list'],
      removedServiceNames: ['drive_abci', 'quorum_list', 'dashmate_helper'],
    });
    expect(dockerCompose.getContainersList).to.have.been.calledOnceWithExactly(config, {
      all: true,
      profiles: ['core', 'platform', 'platform-quorum'],
    });
  });

  it('should find the gateway certificate files', async () => {
    const sslDir = homeDir.joinPath('testnet', 'platform', 'gateway', 'ssl');
    fs.mkdirSync(sslDir, { recursive: true });
    fs.writeFileSync(path.join(sslDir, 'bundle.crt'), 'certificate');

    const { certificateFiles } = await getPlatformArtefacts(config);

    expect(certificateFiles).to.deep.equal([path.join(sslDir, 'bundle.crt')]);
  });

  // A reset that kept data leaves them; one that did not removed them. A
  // volume Docker does not have, or another project's, is neither.
  it('should report the Platform volumes the project created', async () => {
    const owned = { name: 'p_drive_tenderdash', project: 'p', keys: ['drive_tenderdash'] };
    const foreign = { name: 'shared_drive_abci_data', project: 'p', keys: ['drive_abci_data'] };
    dockerCompose.getVolumeCandidates.resolves([owned, foreign]);
    dockerCompose.isVolumeOwned.callsFake(async (candidate) => candidate === owned);

    const { dataVolumes } = await getPlatformArtefacts(config);

    expect(dataVolumes).to.deep.equal(['p_drive_tenderdash']);
  });

  // A restarting or paused container comes back by itself, so it counts as
  // running. Core running is what setup needs, not a reason to refuse.
  it('should report Platform containers in any state but stopped', async () => {
    dockerCompose.getContainersList.resolves([
      { Service: 'core', State: 'running' },
      { Service: 'drive_abci', State: 'restarting' },
      { Service: 'quorum_list', State: 'exited' },
    ]);

    const { activeServices } = await getPlatformArtefacts(config);

    expect(activeServices).to.deep.equal(['drive_abci']);
  });

  it('should fail when Docker cannot be reached rather than report nothing', async () => {
    dockerCompose.throwErrorIfNotInstalled.rejects(new Error('Docker is not running'));

    await expect(getPlatformArtefacts(config)).to.be.rejectedWith('Docker is not running');
  });

  it('should fail when the volumes cannot be read', async () => {
    dockerCompose.getVolumeCandidates.rejects(new Error('permission denied'));

    await expect(getPlatformArtefacts(config)).to.be.rejectedWith('permission denied');
  });

  it('should fail when Docker cannot say whose a volume is', async () => {
    dockerCompose.getVolumeCandidates.resolves([{ name: 'p_drive_tenderdash', project: 'p', keys: ['drive_tenderdash'] }]);
    dockerCompose.isVolumeOwned.rejects(new Error('connect ECONNREFUSED'));

    await expect(getPlatformArtefacts(config)).to.be.rejectedWith('ECONNREFUSED');
  });
});

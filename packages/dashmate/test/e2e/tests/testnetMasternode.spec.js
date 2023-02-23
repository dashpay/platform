const wait = require('@dashevo/dpp/lib/test/utils/wait');
const fs = require('fs');
const isEqual = require('lodash/isEqual');
const { expect } = require('chai');
const fetch = require('node-fetch');
const publicIp = require('public-ip');
const os = require('os');
const prettyMs = require('pretty-ms');
const Docker = require('dockerode');
const StartedContainers = require('../../../src/docker/StartedContainers');
const DockerCompose = require('../../../src/docker/DockerCompose');
const generateBlsKeys = require('../../../src/core/generateBlsKeys');
const { INSIGHT } = require('../lib/constants/insightLinks');
const { SERVICES } = require('../lib/constants/services');
const { STATUS } = require('../lib/constants/statusOutput');
const { execute } = require('../lib/runCommandInCli');
const { getConfig, isConfigExist } = require('../lib/manageConfig');
const { core } = require('../../../configs/system/base');
const { CONFIG_FILE_PATH } = require('../../../src/constants');
const getSelfSignedCertificate = require('../lib/getSelfSignedCertificate');
const { isTestnetServicesRunning } = require('../lib/manageDockerData');
const MasternodeSyncAssetEnum = require('../../../src/enums/masternodeSyncAsset');
const PortStateEnum = require('../../../src/enums/portState');
const ServiceStatusEnum = require('../../../src/enums/serviceStatus');
const TestDashmateClass = require('../lib/testDashmateClass');

describe('Dashmate testnet masternode tests', function main() {
  this.timeout(900000);

  describe('e2e testnet masternode', () => {
    let container;
    let testnetConfig;
    let configEnvs;
    let certificate;
    const testnetNetwork = 'testnet';
    const dashmate = new TestDashmateClass();

    before(() => {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    after(() => {
      fs.unlinkSync(certificate.certificatePath);
      fs.unlinkSync(certificate.privKeyPath);
    });

    it('Setup masternode', async () => {
      const ip = await publicIp.v4();
      const { privateKey: generatedPrivateKeyHex } = await generateBlsKeys();
      certificate = await getSelfSignedCertificate(ip);
      const args = `-i=${ip} -k=${generatedPrivateKeyHex} -s=manual -c=${certificate.certificatePath} -l=${certificate.privKeyPath}`;

      await dashmate.setupTestnet('masternode', args);

      await isTestnetServicesRunning(false, container);

      const isConfig = await isConfigExist(testnetNetwork);
      if (fs.existsSync(CONFIG_FILE_PATH) && isConfig) {
        testnetConfig = await getConfig(testnetNetwork);
        configEnvs = testnetConfig.toEnvs();
      } else {
        throw new Error(`'No configuration file in ${CONFIG_FILE_PATH}`);
      }

      if (configEnvs.CORE_MASTERNODE_ENABLE !== 'true'
        || !configEnvs.CORE_MASTERNODE_OPERATOR_PRIVATE_KEY) {
        throw new Error('This is not masternode configuration. Core masternode parameter is disabled.');
      }
    });

    it('Start testnet nodes', async () => {
      await dashmate.start(testnetNetwork);

      await isTestnetServicesRunning(true, container);
    });

    it('Check core status before core sync process finish', async () => {
      const coreStatus = await dashmate.checkStatus('core');
      const coreOutput = JSON.parse(coreStatus.toString());

      // core versions doesn't maches
      // const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');
      // expect(output.latestVersion).to.be.equal(coreVersion)
      // try {
      //   const latestVersionRes = await fetch('https://api.github.com/repos/dashpay/dash/releases/latest');
      //   const latestVersion = (await latestVersionRes.json()).tag_name.substring(1);
      //   expect(output['Latest version']).to.be.equal(latestVersion)
      // } catch (e) {
      //   throw e;
      // }

      expect(coreOutput.network).to.be.equal(STATUS.testnetNetwork);
      expect(coreOutput.chain).to.be.equal(STATUS.testChain);
      expect(coreOutput.dockerStatus).to.be.equal('running');
      expect(coreOutput.syncAsset).to.be.equal(MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN);

      const peersData = await execute('docker exec dash_masternode_testnet-core-1 dash-cli getpeerinfo');
      const parsedPeersOutput = JSON.parse(peersData.toString());
      const peersNumber = Object.keys(parsedPeersOutput).length;
      expect(coreOutput.peersCount).to.be.equal(peersNumber, 'Peers number are not matching!');

      let peerHeader = 0;
      let peerBlock = 0;
      do {
        for (const peer of parsedPeersOutput) {
          peerBlock = peer.synced_blocks;
          peerHeader = peer.synced_headers;
        }
      } while (peerHeader < 1 && peerBlock < 1);

      expect(coreOutput.p2pService).to.not.be.empty();
      expect(coreOutput.p2pPortState).to.be.equal(PortStateEnum.CLOSED);
      expect(coreOutput.rpcService).to.not.be.empty();

      let coreSyncOutput;
      do {
        coreSyncOutput = JSON.parse(coreStatus.toString());
      } while (+coreSyncOutput.blockHeight < 1 && +coreOutput.headerHeight < 1);

      const explorerBlockHeightRes = await fetch(`${INSIGHT[testnetNetwork]}/status`);
      const { info: { blocks: explorerBlockHeight } } = await explorerBlockHeightRes.json();
      expect(+coreSyncOutput.remoteBlockHeight).to.be.equal(explorerBlockHeight);

      expect(+coreSyncOutput.difficulty).to.be.greaterThan(0);

      expect(coreOutput.serviceStatus).to.be.equal(ServiceStatusEnum.syncing);
      if (!(coreOutput.verificationProgress > 0 && coreOutput.verificationProgress <= 1)) {
        throw new Error(`Invalid status output for syncing process: ${coreOutput.verificationProgress}% `);
      }
    });

    it('Check platform status before core sync process finish', async () => {
      const platformStatus = await dashmate.checkStatus('platform');
      expect(platformStatus.substring(0, platformStatus.length - 1)).to.equal('Platform status is not available until core sync is complete!');
    });

    it('Check services status before core sync process finish', async () => {
      const servicesStatus = await dashmate.checkStatus('services');
      const output = JSON.parse(servicesStatus.toString());

      const listIDs = await execute('docker ps --format "{{ .ID }}"');
      const containerIds = listIDs.toString().split('\n');
      containerIds.pop();

      for (const serviceData of output) {
        expect(Object.keys(SERVICES)).to.include(serviceData.service);
        expect(serviceData.status).to.equal('running');
        expect(listIDs).to.include(serviceData.containerId);
      }
    });

    it('Verify status overview before core sync process finish', async () => {
      const overviewStatus = await dashmate.checkStatus('');
      const output = JSON.parse(overviewStatus.toString());
      const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|-(.*)/g, '');

      expect(output.Network).to.equal(STATUS.test);
      expect(output['Core Version']).to.equal(coreVersion);
      expect(output['Core Status']).to.include(ServiceStatusEnum.syncing);
      expect(output['Masternode Status']).to.equal(STATUS.masternode_status);
      expect(output['Platform Status']).to.equal(STATUS.platform_status);
    });

    it('Check host status before core sync process finish', async () => {
      const hostStatus = await dashmate.checkStatus('host');
      const output = JSON.parse(hostStatus.toString());

      expect(output.hostname).to.equal(os.hostname());
      expect(output.uptime).to.include(prettyMs(os.uptime() * 1000, { unitCount: 2 }));
      expect(output.platform).to.equal(os.platform());
      expect(output.arch).to.equal(os.arch());
      expect(output.username).to.equal(os.userInfo().username);
      expect(output.cpus).to.equal(os.cpus().length);
      expect(output.ip).to.equal(await publicIp.v4());
    });

    it('Check masternode status before core sync process finish', async () => {
      const masternodeStatus = await dashmate.checkStatus('masternode');
      const output = JSON.parse(masternodeStatus.toString());

      // expect(output['Masternode status']).to.equal(STATUS.masternode_status); bugged
      expect(output.sentinel.state).to.equal(STATUS.sentinel_statusNotSynced);
    });

    it('Restart testnet network before core sync process finish', async () => {
      const coreStatus = await dashmate.checkStatus('core');
      const statusBeforeRestart = JSON.parse(coreStatus);

      await dashmate.restart(testnetNetwork);

      await isTestnetServicesRunning(true, container);

      const statusAfterRestart = JSON.parse(coreStatus.toString());
      if (+statusBeforeRestart.blockHeight !== +statusAfterRestart.blockHeight) {
        throw new Error('Block height is different after restart.');
      } else {
        let blockHeightSync;
        do {
          await wait(5000);
          blockHeightSync = JSON.parse(coreStatus.toString());
        } while (+blockHeightSync.blockHeight <= +statusAfterRestart.blockHeight);
      }

      const restartConfig = getConfig(testnetNetwork);
      expect(isEqual(restartConfig, testnetConfig)).to.equal(true);
    });

    it('Stop testnet network', async () => {
      await dashmate.stop(testnetNetwork);

      await isTestnetServicesRunning(false, container);
    });

    it('Start again testnet network', async () => {
      await dashmate.start(testnetNetwork);

      await isTestnetServicesRunning(true, container);
    });

    it('Reset testnet network', async () => {
      const coreStatus = await dashmate.checkStatus('core');
      const statusBeforeReset = JSON.parse(coreStatus);

      await dashmate.stop(testnetNetwork);
      await isTestnetServicesRunning(false, container);

      await dashmate.reset(testnetNetwork);
      await isTestnetServicesRunning(false, container);
      const resetTestnetConfig = getConfig(testnetNetwork);
      expect(isEqual(resetTestnetConfig, testnetConfig)).to.equal(true);

      await dashmate.start(testnetNetwork);
      await isTestnetServicesRunning(true, container);

      const statusAfterReset = JSON.parse(coreStatus);
      if (!(+statusAfterReset.headerHeight < +statusBeforeReset.headerHeight)
        && (+statusAfterReset.blockHeight !== 0)) {
        throw new Error('Core sync data have not been reset.');
      } else {
        let headerHeighSync;
        do {
          await wait(5000);
          headerHeighSync = JSON.parse(coreStatus.toString());
        } while (+headerHeighSync.headerHeight < 1000);
      }
    });
  });
});

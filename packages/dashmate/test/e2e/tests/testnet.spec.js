const Docker = require('dockerode');
const StartedContainers = require('../../../src/docker/StartedContainers');
const DockerCompose = require('../../../src/docker/DockerCompose');

const fs = require("fs");
const isEqual = require('lodash/isEqual');
const { expect } = require('chai');
const fetch = require("node-fetch");
const publicIp = require('public-ip');
const os = require("os");
const prettyByte = require('pretty-bytes');
const prettyMs = require('pretty-ms');
const generateBlsKeys = require("../../../src/core/generateBlsKeys");
const wait = require("@dashevo/dpp/lib/test/utils/wait");

const { EMPTY_TESTNET_CONFIG_FIELDS } = require('../lib/constants/configFields')
const { INSIGHT_URLs } = require('../lib/constants/insightLinks')
const { SERVICES } = require('../lib/constants/services')
const {STATUS} = require("../lib/constants/statusOutput");

const { execute } = require('../../e2e/lib/runCommandInCli')
const { getConfig, isConfigExist } = require("../../e2e/lib/manageConfig");
const { core } = require("../../../configs/system/base");
const {CONFIG_FILE_PATH} = require("../../../src/constants");
const getSelfSignedCertificate = require("../lib/getSelfSignedCertificate");
const { isTestnetServicesRunning } = require("../lib/manageDockerData");
const MasternodeSyncAssetEnum = require("../../../src/enums/masternodeSyncAsset");
const PortStateEnum = require("../../../src/enums/portState");
const ServiceStatusEnum = require("../../../src/enums/serviceStatus");
const TestDashmateClass = require("../lib/testDashmateClass");

describe('Testnet dashmate', function main() {
  this.timeout(900000);

  describe('e2e testnet network', async function () {
    let container;
    let testnetConfig, configEnvs;
    const testnetNetwork = 'testnet';
    let certificate;
    const dashmate = new TestDashmateClass();

    before(function () {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    after(async function () {
      fs.unlinkSync(certificate.certificatePath);
      fs.unlinkSync(certificate.privKeyPath);
    });

    it('Setup testnet nodes', async () => {
      const ip = await publicIp.v4();
      const {privateKey: generatedPrivateKeyHex} = await generateBlsKeys();
      certificate = await getSelfSignedCertificate(ip)

      await dashmate.setupTestnet('masternode', [`-i=${ip} -k=${generatedPrivateKeyHex} -s=manual -c=${certificate.certificatePath} -l=${certificate.privKeyPath}`])

      await isTestnetServicesRunning(false, container)

      const isConfig = await isConfigExist(testnetNetwork)
      if (fs.existsSync(CONFIG_FILE_PATH) && isConfig) {
        testnetConfig = await getConfig(testnetNetwork)
        configEnvs = testnetConfig.toEnvs(); //delete if checking list of envs is not needed
      } else {
        throw new Error('No configuration file in ' + CONFIG_FILE_PATH);
      }

      // waiting for a list of envs that should not be empty after setup
      // for (let key in configEnvs) {
      //   if (!configEnvs[key]) {
      //     const checkKeyInArray = EMPTY_TESTNET_CONFIG_FIELDS.includes(key);
      //     expect(checkKeyInArray).to.equal(true, key + ' should not be empty.');
      //   }
      // }
    });

    it('Start testnet nodes', async () => {
      await dashmate.start(testnetNetwork)

      await isTestnetServicesRunning(true, container)
    });

    it('Check core status before core sync process finish', async () => {
      const coreStatus = await dashmate.checkStatus('core')
      const coreOutput = JSON.parse(coreStatus.toString())

      //update core version in config because versions don't match??
      // const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');
      // expect(output.latestVersion).to.be.equal(coreVersion)
      // try {
      //   const latestVersionRes = await fetch('https://api.github.com/repos/dashpay/dash/releases/latest');
      //   const latestVersion = (await latestVersionRes.json()).tag_name.substring(1);
      //   expect(output['Latest version']).to.be.equal(latestVersion)
      // } catch (e) {
      //   throw e;
      // }

      expect(coreOutput.network).to.be.equal(STATUS.testnetNetwork)
      expect(coreOutput.chain).to.be.equal(STATUS.testChain)
      expect(coreOutput.dockerStatus).to.be.equal('running')
      expect(coreOutput.syncAsset).to.be.equal(MasternodeSyncAssetEnum.MASTERNODE_SYNC_BLOCKCHAIN)

      const peersData = await execute('docker exec dash_masternode_testnet-core-1 dash-cli getpeerinfo').then(peers => {
        return JSON.parse(peers.toString());
      })
      const peersNumber = Object.keys(peersData).length;
      expect(coreOutput.peersCount).to.be.equal(peersNumber, 'Peers number are not matching!')

      let peerHeader, peerBlock;
      do {
        for (const peer of peersData) {
          peerBlock = peer['synced_blocks']
          peerHeader = peer['synced_headers']
          console.log(`Debug peers block / headers: ${peerBlock} / ${peerHeader}`)
        }
      } while (peerHeader < 1 && peerBlock < 1)

      expect(coreOutput.p2pService).to.not.be.empty()
      expect(coreOutput.p2pPortState).to.be.equal(PortStateEnum.CLOSED)
      expect(coreOutput.rpcService).to.not.be.empty()

      let coreSyncOutput;
      do {
        coreSyncOutput = JSON.parse(coreStatus.toString());
      } while (+coreSyncOutput.blockHeight < 1 && +coreOutput.headerHeight < 1)

      let explorerBlockHeightRes = await fetch(`${INSIGHT_URLs[testnetNetwork]}/status`);
      ({info: {blocks: explorerBlockHeight}} = await explorerBlockHeightRes.json());
      expect(+coreSyncOutput.remoteBlockHeight).to.be.equal(explorerBlockHeight)

      expect(+coreSyncOutput.difficulty).to.be.greaterThan(0)

      expect(coreOutput.serviceStatus).to.be.equal(ServiceStatusEnum.syncing)
      if (!(coreOutput.verificationProgress > 0 && coreOutput.verificationProgress <= 100)) {
        throw new Error(`Invalid status output for syncing process: ${coreOutput.verificationProgress}% `)
      }
    });

    it('Check platform status before core sync process finish', async () => {
      await dashmate.checkStatus('platform').then(res => {
        expect(res.substring(0, res.length - 1)).to.equal('Platform status is not available until core sync is complete!')
      })
    });

    it('Check services status before core sync process finish', async () => {
      const servicesStatus = await dashmate.checkStatus('services');
      const output = JSON.parse(servicesStatus.toString())

      const listIDs = await execute('docker ps --format "{{ .ID }}"').then(ids => {
        const containerIds = ids.toString().split('\n');
        containerIds.pop()
        return containerIds;
      })

      for (let serviceData of output) {
        expect(Object.keys(SERVICES)).to.include(serviceData.service);
        expect(serviceData.status).to.equal('running');
        expect(listIDs).to.include(serviceData.containerId)
      }
    })

    it('Verify status overview before core sync process finish', async () => {
      const overviewStatus = await dashmate.checkStatus('') //status overview
      const output = JSON.parse(overviewStatus.toString())
      const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');

      expect(output.Network).to.equal(STATUS.test)
      expect(output['Core Version']).to.equal(coreVersion)
      expect(output['Core Status']).to.include(ServiceStatusEnum.syncing)
      expect(output['Masternode Status']).to.equal(STATUS.masternode_status)
      expect(output['Platform Status']).to.equal(STATUS.platform_status)
    })

    it('Check host status before core sync process finish', async () => {
      const hostStatus = await dashmate.checkStatus('host')
      const output = JSON.parse(hostStatus.toString())

      expect(output.hostname).to.equal(os.hostname());
      expect(output.uptime).to.include(prettyMs(os.uptime() * 1000, {unitCount: 2}));
      expect(output.platform).to.equal(os.platform());
      expect(output.arch).to.equal(os.arch());
      expect(output.username).to.equal(os.userInfo().username);
      // expect(output.diskFree).to.equal(0); //bugged
      // expect(output.memory).to.equal(`${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`); //doesn't work properly on wsl
      expect(output.cpus).to.equal(os.cpus().length);
      expect(output.ip).to.equal(await publicIp.v4());
    });

    it('Check masternode status before core sync process finish', async () => {
      const masternodeStatus = await dashmate.checkStatus('masternode')
      const output = JSON.parse(masternodeStatus.toString())

      // expect(output['Masternode status']).to.equal(STATUS.masternode_status); //bugged
      expect(output.sentinel.state).to.equal(STATUS.sentinel_statusNotSynced);
      })

    it('Restart testnet network before core sync process finish', async () => {
      const coreStatus = await dashmate.checkStatus('core')
      const statusBeforeRestart = JSON.parse(coreStatus);

      await dashmate.restart(testnetNetwork)

      await isTestnetServicesRunning(true, container)

      const statusAfterRestart = JSON.parse(coreStatus.toString());
      if (+statusBeforeRestart.blockHeight !== +statusAfterRestart.blockHeight) {
        throw new Error('Block height is different after restart.')
      } else {
        let blockHeighSync;
        do {
          await wait(5000);
          blockHeighSync = JSON.parse(coreStatus.toString());
        } while (+blockHeighSync.blockHeight <= +statusAfterRestart.blockHeight)
      }

      const restartConfig = getConfig(testnetNetwork);
      expect(isEqual(restartConfig, testnetConfig)).to.equal(true);
    });

    it('Stop testnet network', async () => {
      await dashmate.stop(testnetNetwork)

      await isTestnetServicesRunning(false, container)
    });

    it('Start again testnet network', async () => {
      await dashmate.start(testnetNetwork)

      await isTestnetServicesRunning(true, container)
    });

    it('Reset testnet network', async () => {
      const coreStatus = await dashmate.checkStatus('core')
      const statusBeforeReset = JSON.parse(coreStatus);

      await dashmate.stop(testnetNetwork)
      await isTestnetServicesRunning(false, container)

      await dashmate.reset(testnetNetwork)
      await isTestnetServicesRunning(false, container)
      const resetTestnetConfig = getConfig(testnetNetwork);
      expect(isEqual(resetTestnetConfig, testnetConfig)).to.equal(true);

      await dashmate.start(testnetNetwork)
      await isTestnetServicesRunning(true, container)

      const statusAfterReset = JSON.parse(coreStatus);
      if (!(+statusAfterReset.headerHeight < +statusBeforeReset.headerHeight) &&
        (+statusAfterReset.blockHeight !== 0)) {
        throw new Error('Core sync data have not been reset.')
      } else {
        let headerHeighSync;
        do {
          await wait(5000);
          headerHeighSync = JSON.parse(coreStatus.toString());
        } while (+headerHeighSync.headerHeight < 1000)
      }
    });
  });
});

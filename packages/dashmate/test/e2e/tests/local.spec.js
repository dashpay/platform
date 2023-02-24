const fs = require('fs');
const { expect } = require('chai');
const isEqual = require('lodash/isEqual');
const Docker = require('dockerode');
const StartedContainers = require('../../../src/docker/StartedContainers');
const DockerCompose = require('../../../src/docker/DockerCompose');
const { getConfig, isConfigExist } = require('../lib/manageConfig');
const { isGroupServicesRunning } = require('../lib/manageDockerData');
const { platform } = require('../../../configs/system/base');
const { CONFIG_FILE_PATH } = require('../../../src/constants');
const TestDashmateClass = require('../lib/testDashmateClass');
const { execute } = require('../lib/runCommandInCli');

describe('Local dashmate', function main() {
  this.timeout(900000);

  describe('e2e local network', () => {
    let container;
    let localConfig;
    let nodes;
    const localNetwork = 'local';
    const dashmate = new TestDashmateClass();

    before(() => {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    it('Setup local group nodes', async () => {
      // await dashmate.setupLocal(nodes = 3);

      await isGroupServicesRunning(false, container);

      const isConfExist = await isConfigExist(localNetwork);
      if (fs.existsSync(CONFIG_FILE_PATH) && isConfExist) {
        localConfig = await getConfig(localNetwork);
      } else {
        throw new Error(`'No configuration file in ${CONFIG_FILE_PATH}`);
      }
    });

    it('Start local group nodes', async () => {
      await dashmate.start(localNetwork);

      await isGroupServicesRunning(true, container);
    });

    it('Check group list', async () => {
      const listOutput = await dashmate.getGroupStatus('list');

      const arr = [];
      for (let i = 0; i < nodes; i++) {
        arr.push(`local_${i + 1}`, `local node #${i + 1}`);
      }
      arr.push('local_seed', 'seed node for local network');

      arr.forEach((node) => {
        expect(listOutput).to.include(node, `List of group nodes does not contain expected ${node}! \n`);
      });
    });

    it('Check group status', async () => {
      const platformVersion = platform.drive.tenderdash.docker.image.replace(/\/|\(.*?\)|dashpay|tenderdash:/g, '');

      const blockchainInfo = await execute('docker exec dash_masternode_local_seed-core-1 dash-cli getblockchaininfo');
      const blockHeight = JSON.parse(blockchainInfo.toString());

      const statusOutput = await dashmate.getGroupStatus('status --format=json');
      const parse = JSON.parse(statusOutput);

      for (const node of parse) {
        expect(node.network).to.be.equal('local');
        expect(node.core.serviceStatus).to.be.equal('up');
        expect(node.core.blockHeight).to.be.at.least(blockHeight.blocks);

        if (node.platform.tenderdash === null) {
          expect(node.platform.enabled).to.be.equal(false);
          break;
        }

        expect(node.platform.enabled).to.be.equal(true);
        expect(node.platform.tenderdash.serviceStatus).to.be.equal('up');
        expect(node.platform.tenderdash.version).to.be.equal(platformVersion);
        expect(+node.platform.tenderdash.lastBlockHeight).to.be.at.least(1);
        expect(+node.platform.tenderdash.peers).to.be.at.least(2);
        expect(node.platform.tenderdash.network).to.include('dash_masternode_local');
      }
    });

    it('Stop local group nodes', async () => {
      await dashmate.stop(localNetwork);

      await isGroupServicesRunning(false, container);

      const status = await dashmate.getGroupStatus('status');
      expect(status).to.be.empty();
    });

    it('Start again local group nodes', async () => {
      await dashmate.start(localNetwork);

      await isGroupServicesRunning(true, container);

      const status = await dashmate.getGroupStatus('status');
      expect(status).to.not.be.empty();
    });

    it('Restart local group nodes', async () => {
      await dashmate.restart(localNetwork);

      await isGroupServicesRunning(true, container);

      const status = await dashmate.getGroupStatus('status');
      expect(status).to.not.be.empty();

      if (await isConfigExist(localNetwork)) {
        const restartConfig = await getConfig(localNetwork);
        expect(isEqual(restartConfig, localConfig)).to.equal(true, 'Local config is different after restart.');
      } else {
        throw new Error('There is no local config after restart.');
      }
    });

    it.only('Reset local group nodes', async () => {
      const status = await dashmate.getGroupStatus('status');

      await dashmate.stop(localNetwork);

      await dashmate.reset(localNetwork);
      await isGroupServicesRunning(false, container);
      expect(status).to.be.empty();

      if (await isConfigExist(localNetwork)) {
        const resetConfig = await getConfig(localNetwork);
        expect(isEqual(resetConfig, localConfig)).to.equal(false, 'Local config is the same after restart.');
      } else {
        throw new Error('There is no local config after restart.');
      }
    });

    it('Start local group nodes after reset', async () => {
      await dashmate.start(localNetwork);

      await isGroupServicesRunning(true, container);

      const status = await dashmate.getGroupStatus('status');
      expect(status).to.not.be.empty();
    });
  });
});

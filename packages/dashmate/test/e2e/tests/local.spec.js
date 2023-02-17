const fs = require('fs');
const { expect } = require('chai');
const isEqual = require('lodash/isEqual');
const Docker = require('dockerode');
const StartedContainers = require('../../../src/docker/StartedContainers');
const DockerCompose = require('../../../src/docker/DockerCompose');
const { getConfig, isConfigExist } = require('../lib/manageConfig');
const { isGroupServicesRunning } = require('../lib/manageDockerData');
const { core, platform } = require('../../../configs/system/base');
const { CONFIG_FILE_PATH } = require('../../../src/constants');
const TestDashmateClass = require('../lib/testDashmateClass');

describe('Local dashmate', function main() {
  this.timeout(900000);

  describe('e2e local network', () => {
    let container;
    let localConfig;
    let nodes;
    const localNetwork = 'local'
    const dashmate = new TestDashmateClass();

    before(() => {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    it('Setup local group nodes', async () => {
      await dashmate.setupLocal(nodes = 3);

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

      await isGroupServicesRunning(true, localConfig, container);
    });

    it('Check group list', async () => {
      const listOutput = await dashmate.checkGroupStatus('list');

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
      const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');
      const platformVersion = platform.drive.tenderdash.docker.image.replace(/\/|\(.*?\)|dashpay|tenderdash:/g, '');

      const statusOutput = await dashmate.checkGroupStatus('list');

      const arrayOfResults = statusOutput.split(/\n/);
      arrayOfResults.pop();

      const output = arrayOfResults.map((n, index) => index % 2 === 0 ? n : JSON.parse(n));

      let nodeIndex = 1;
      for (let i = 0; i < output.length; i++) {
        if (typeof output[i] === 'string') {
          if (i === output.length - 2) {
            expect(output[i]).to.be.equal('Node local_seed');
            continue;
          }
          expect(output[i]).to.be.equal(`Node local_${nodeIndex}`);
          nodeIndex++;
        } else if (typeof output[i] === 'object') {
          expect(output[i].Network).to.be.equal('regtest');
          expect(output[i]['Core Version']).to.be.equal(coreVersion);
          expect(output[i]['Core Status']).to.be.equal('running');
          if (i === output.length - 1) {
            break;
          }
          expect(output[i]['Masternode Status']).to.be.equal('Ready');
          expect(output[i]['Platform Version']).to.be.equal(platformVersion);
          expect(output[i]['Platform Status']).to.be.equal('running');
        } else {
          throw new Error('Group status data conversion went wrong!');
        }
      }
    });

    it('Stop local group nodes', async () => {
      await dashmate.stop(localNetwork);

      await isGroupServicesRunning(false, container);

      await dashmate.checkGroupStatus('status').then(async (res) => {
        expect(res).to.be.empty();
      });
    });

    it('Start again local group nodes', async () => {
      await dashmate.start(localNetwork);

      await isGroupServicesRunning(true, container);

      await dashmate.checkGroupStatus('status').then(async (res) => {
        expect(res).to.not.be.empty();
      });
    });

    it('Restart local group nodes', async () => {
      await dashmate.restart(localNetwork);

      await isGroupServicesRunning(true, container);

      await dashmate.checkGroupStatus('status').then(async (res) => {
        expect(res).to.not.be.empty();
      });

      if (await isConfigExist(localNetwork)) {
        const restartConfig = await getConfig(localNetwork);
        expect(isEqual(restartConfig, localConfig)).to.equal(true, 'Local config is different after restart.');
      } else {
        throw new Error('There is no local config after restart.');
      }
    });

    it('Reset local group nodes', async () => {
      const status = await dashmate.checkGroupStatus('status');

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

      await dashmate.checkGroupStatus('status').then(async (res) => {
        expect(res).to.not.be.empty();
      });
    });
  });
});

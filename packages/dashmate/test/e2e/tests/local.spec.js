const Docker = require('dockerode');
const StartedContainers = require('../../src/docker/StartedContainers');
const DockerCompose = require('../../src/docker/DockerCompose');

const { isGroupConfigExist, getGroupConfig } = require("../lib/test/manageConfig");
const { removeContainers, removeVolumes, isGroupServicesRunning } = require("../lib/test/manageDockerData");
const { core, platform } = require("../../configs/system/base");
const fs = require("fs");
const { expect } = require("chai");
const { CONFIG_FILE_PATH } = require("../../src/constants");
const { EMPTY_LOCAL_CONFIG_FIELDS } = require("../lib/test/constants/constants");
const { execute } = require('../lib/test/commandRunner')

describe.skip('Local network', function main() {
  this.timeout(900000);

  describe('Local network', function () {
    let container;
    let localConfig;
    const nodes = 3;
    const localNetwork = 'local'
    const minerInterval = '2.5m'

    before(function () {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    after(async function () {
      await removeContainers(localNetwork, container)
      await removeVolumes(localNetwork, container)
      await localConfig.removeConfig(localNetwork);
    });

    it('Setup local group nodes', async () => {
      await execute(`dashmate setup ${localNetwork} --node-count=${nodes} --debug-logs --miner-interval=${minerInterval}`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const isConfExist = await isGroupConfigExist(localNetwork)
      if (fs.existsSync(CONFIG_FILE_PATH) && isConfExist) {
        localConfig = await getGroupConfig(localNetwork)
      } else {
        throw new Error('No configuration file: ' + CONFIG_FILE_PATH);
      }

      for (let i = 0; i <= localConfig.length - 1; i++) {
        const envs = localConfig[i].toEnvs();
        for (let key in envs) {
          if (!envs[key]) {
            const checkKeyInArray = EMPTY_LOCAL_CONFIG_FIELDS.includes(key);
            expect(checkKeyInArray).to.equal(true, key + ' should not be empty.');
          }
        }
      }

      await isGroupServicesRunning(false, localConfig, container)
    });

    it('Start local group nodes', async () => {

      await execute(`yarn dashmate group start --verbose`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      await isGroupServicesRunning(true, localConfig, container)
    });

    it('Check group list', async () => {
      await execute(`yarn dashmate group list`).then( res => {
        const data = res.toString();

        let arr = []
        for (let i = 0; i < nodes; i++){
          arr.push(`local_${i + 1}`, `local node #${i + 1}`)
        }

        arr.forEach(node => {
          expect(data).to.include(node);
        })
        expect(data).to.include('local_seed');
        expect(data).to.include('seed node for local network');
      })
    });

    it('Check group status', async () => {
      const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');
      const platformVersion = platform.drive.tenderdash.docker.image.replace(/\/|\(.*?\)|dashpay|tenderdash:/g, '')

      await execute(`yarn dashmate group status --format=json`).then( res => {
        const resString = res.toString();
        const arrayOfResults = resString.split(/\n/)
        arrayOfResults.pop();

        let output = arrayOfResults.map((n, index) => index % 2 === 0 ? n : JSON.parse(n));

        let node_index = 1;
        for(let i = 0; i < output.length; i++) {
          if(typeof output[i] === 'string') {
            if(i === output.length - 2) {
              expect(output[i]).to.be.equal('Node local_seed')
              continue;
            }
            expect(output[i]).to.be.equal(`Node local_${node_index}`)
            node_index++;
          } else if(typeof output[i] === 'object') {
            expect(output[i].Network).to.be.equal('regtest')
            expect(output[i]['Core Version']).to.be.equal(coreVersion)
            expect(output[i]['Core Status']).to.be.equal('running')
            if(i === output.length - 1) { break }
            expect(output[i]['Masternode Status']).to.be.equal('Ready')
            expect(output[i]['Platform Version']).to.be.equal(platformVersion)
            expect(output[i]['Platform Status']).to.be.equal('running')
          } else {
            throw new Error('Group status conversion data went wrong!')
          }
        }
      });
    });

    it('Stop local group nodes', async () => {
      await execute(`yarn dashmate group stop --verbose`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const isConfigExist = await isGroupConfigExist(localNetwork)
      expect(isConfigExist).to.equal(true);

      await isGroupServicesRunning(false, localConfig, container)
    });

    it('Start again local group nodes', async () => {
      await execute(`yarn dashmate group start --verbose`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      await isGroupServicesRunning(true, localConfig, container)
    });

    it('Restart local group nodes', async () => {
      await execute(`yarn dashmate group restart --verbose`).then( res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const isConfigExist = await isGroupConfigExist(localNetwork)
      expect(isConfigExist).to.equal(true);
      if (isConfigExist) {
        let restartConfig = await getGroupConfig(localNetwork);
        expect(isEqual(restartConfig, localConfig)).to.equal(true, 'Local config is different after restart group of nodes');
      } else {
        throw new Error('There is no local config after restart')
      }

      await isGroupServicesRunning(true, localConfig, container)
    });

    it('Reset local group nodes', async () => {
      await execute(`yarn dashmate group reset --verbose`).then( res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const isConfExist = await isGroupConfigExist(localNetwork)
      if(!isConfExist) {
        throw new Error(`${localNetwork} config file has been deleted!`)
      }

      await isGroupServicesRunning(false, localConfig, container)

      await execute(`yarn dashmate group start --verbose`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const restartGroupConfig = await getGroupConfig(localNetwork);
      expect(isEqual(restartGroupConfig, localConfig)).to.equal(true);

      await isGroupServicesRunning(true, localConfig, container)
    });
  });
});

const Docker = require('dockerode');
const StartedContainers = require('../../src/docker/StartedContainers');
const DockerCompose = require('../../src/docker/DockerCompose');
const fs = require("fs");
const isEqual = require('lodash.isequal');
const { expect } = require('chai');
const fetch = require("node-fetch");
const publicIp = require('public-ip');
const { EMPTY_TESTNET_CONFIG_FIELDS, INSIGHT_URLs, SERVICES } = require('../lib/test/constants/constants')
const CoreService = require("../../src/core/CoreService");
const createRpcClient = require("../../src/core/createRpcClient");
const { removeVolumes, removeContainers } = require('../lib/test/manageDockerData')
const generateBlsKeys = require("../../src/core/generateBlsKeys");
const { execute } = require('../lib/test/commandRunner')
const { getConfig } = require("../lib/test/manageConfig");
const { core } = require("../../configs/system/base");
const os = require("os");
const prettyByte = require('pretty-bytes');
const prettyMs = require('pretty-ms');
const wait = require("@dashevo/dpp/lib/test/utils/wait");
const {CONFIG_FILE_PATH} = require("../../src/constants");
const {STATUS} = require("../lib/test/constants/statusesConstants");


describe.skip('Testnet network', function main() {
  this.timeout(900000);

  describe('Testnet network', function () {
    let container;
    let testnetConfig, configEnvs;
    const testnetNetwork = 'testnet';

    before(function () {
      const dockerode = new Docker();
      const startedContainers = new StartedContainers();
      container = new DockerCompose(dockerode, startedContainers);
    });

    after(async function () {
      await removeContainers(testnetConfig, container)
      await removeVolumes(testnetConfig, container)
      await testnetConfig.removeConfig(testnetNetwork);
    });

    it('Setup testnet nodes', async () => {
      const ip = await publicIp.v4();
      const { privateKey: generatedPrivateKeyHex } = await generateBlsKeys();
      const nodeType = 'masternode';
      const nodes = 3;
      const minerInterval = '2.5m'

      await execute(`yarn dashmate setup ${testnetNetwork} ${nodeType} -i=${ip} -k=${generatedPrivateKeyHex} --node-count=${nodes} --debug-logs --miner-interval=${minerInterval}`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const isConfig = await isConfigExist(testnetNetwork)
      if (fs.existsSync(CONFIG_FILE_PATH) && isConfig) {
        testnetConfig = await getConfig(testnetNetwork)
        configEnvs = testnetConfig.toEnvs();
      } else {
        throw new Error('No configuration file: ' + CONFIG_FILE_PATH);
      }

      for (let key in configEnvs) {
        if (!configEnvs[key]) {
          const checkKeyInArray = EMPTY_TESTNET_CONFIG_FIELDS.includes(key);
          expect(checkKeyInArray).to.equal(true, key + ' should not be empty.');
        }
      }

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(false, `${key} is running!`)
      }
    });

    it('Start testnet nodes', async () => {
      await execute(`yarn dashmate start`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(true, `${key} is not running!`)
      }
    });

    it('Check core status before core sync process finish', async () => {
      let peerHeader, peerBlock;
      testnetConfig = await getConfig('testnet')

      await execute(`yarn dashmate status core --format=json`).then( async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }

        const output = JSON.parse(res.toString())

        const coreVersion = core.docker.image.replace(/\/|\(.*?\)|dashpay|dashd:|\-(.*)/g, '');
        expect(output.Version).to.be.equal(coreVersion)

        try {
          const latestVersionRes = await fetch('https://api.github.com/repos/dashpay/dash/releases/latest');
          const latestVersion = (await latestVersionRes.json()).tag_name.substring(1);
          expect(output['Latest version']).to.be.equal(latestVersion)
        } catch (e) {
          throw new e
        }

        expect(output.Network).to.be.equal(STATUS.test)


        expect(output['Sync asset']).to.be.equal(STATUS.sync_asset)

        await execute('docker exec dash_masternode_testnet-core-1 dash-cli getpeerinfo').then(peers => {
          const peersOutput = JSON.parse(peers.toString())
          const peersNumber = Object.keys(peersOutput).length;
          expect(output['Peer count']).to.be.equal(peersNumber, 'Peers number are not matching!')

          do {
            for (const peer of peersOutput) {
              peerHeader = peer['synced_headers']
              peerBlock = peer['synced_blocks']
            }
          } while (peerHeader < 1 && peerBlock < 1)
        })

        // expect(output['P2P service']).to.be.equal()
        // expect(output['P2P port']).to.be.equal()
        // expect(output['RPC service']).to.be.equal()

        const coreService = await getCoreService(testnetConfig, container);
        do {
          let explorerBlockHeightRes = await fetch(`${INSIGHT_URLs[testnetNetwork]}/status`);
          ({ info: { blocks: explorerBlockHeight } } = await explorerBlockHeightRes.json());
          ({ result: { headers: coreHeaders } } = await coreService.getRpcClient().getBlockchainInfo());

          if(coreHeaders < explorerBlockHeight) {
            await wait(5000);
          }
        } while (coreHeaders !== explorerBlockHeight)

        expect(+output['Header height']).to.be.greaterThan(0)
        expect(+output['Block height']).to.be.greaterThan(0)

        expect(output.Status).to.contain('syncing')
        const status = (output.Status).split(/[ %]/)
        if (!(+status[1] >= 0 && +status[1] <= 100)) {
          throw new Error(`Invalid status output for syncing process: ${status[1]}% `)
        }

        // let explorerBlockHeightRes = await fetch(`${INSIGHT_URLs[testnetNetwork]}/status`);
        // ({ info: { difficulty: blocksDifficulty } } = await explorerBlockHeightRes.json());
        expect(+output.Difficulty).to.be.greaterThan(0)

        const sentinelVersion = core.sentinel.docker.image.replace(/\/|\(.*?\)|dashpay|sentinel:|\-(.*)/g, '');
        expect(output['Sentinel version']).to.be.equal(sentinelVersion)
        expect(output['Sentinel status']).to.be.equal(STATUS.sentinel_status)
        expect(output['Remote block height']).to.be.equal(explorerBlockHeight)
      })
    });

    it('Check platform status before core sync process finish', async () => {
      await execute(`yarn dashmate status platform`).then(res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        } else {
          const output = res.toString()
          expect(output.substring(0, output.length-1)).to.equal('Platform status is not available until core sync is complete!')
        }
      })
    });

    it('Check services status before core sync process finish', async () => {
      await execute(`yarn dashmate status services --format=json`).then(async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }

        let containerIds;
        await execute('docker ps --format "{{ .ID }}"').then(ids => {
          containerIds = ids.toString().split('\n');
          containerIds.pop() //empty item
        })

        const services = ['Core', 'Sentinel', 'Drive ABCI', 'Drive Tenderdash', 'DAPI API', 'DAPI Transactions Filter Stream', 'DAPI Envoy']
        const output = JSON.parse(res.toString())
        for (let serviceData of output) {
          expect(services).to.include(serviceData.Service);
          expect(serviceData.Status).to.equal('running');
          expect(containerIds).to.include(serviceData['Container ID'])
        }
      })
    });

    it('Verify status overview before core sync process finish', async () => {
      await execute(`yarn dashmate status --format=json`).then(async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }

        const output = JSON.parse(res.toString())
        expect(output.Network).to.equal(STATUS.test)
        expect(output['Core Version']).to.equal('18.1.0')
        // expect(output['Core Status']).to.include('syncing')
        expect(output['Masternode Status']).to.equal(STATUS.masternode_status)
        expect(output['Platform Status']).to.equal(STATUS.platform_status)
      })
    });

    it('Check host status before core sync process finish', async () => {
      await execute(`yarn dashmate status host --format=json`).then(async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }

        const output = JSON.parse(res.toString())
        expect(output.Hostname).to.equal(os.hostname());
        expect(output.Uptime).to.include(prettyMs(os.uptime() * 1000, {unitCount:2}));
        expect(output.Platform).to.equal(os.platform());
        expect(output.Arch).to.equal(os.arch());
        expect(output.Username).to.equal(os.userInfo().username);
        expect(output.Diskfree).to.equal(0); //hard-coded
        // expect(output.Memory).to.equal(`${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`);
        expect(output.CPUs).to.equal(os.cpus().length);
        expect(output.IP).to.equal(await publicIp.v4());
      })
    });

    it('Check masternode status before core sync process finish', async () => {
      await execute(`yarn dashmate status masternode --format=json`).then(async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }

        const output = JSON.parse(res.toString())
        expect(output['Masternode status']).to.equal(STATUS.masternode_status);
        expect(output['Sentinel status']).to.equal(STATUS.sentinel_status);
      })
    });

    it('Restart testnet network before core sync process finish', async () => {
      let output;

      await execute(`yarn dashmate restart`).then( async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      do {
        const status = await execute(`yarn dashmate status core --format=json`);
        output = JSON.parse(status.toString())
        await wait(5000);
      } while (+output['Header height'] < 1)

      const restartConfig = getConfig(testnetNetwork);
      expect(isEqual(restartConfig, testnetConfig)).to.equal(true);

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(true, `${key} is not running!`)
      }
    });

    it('Stop testnet network', async () => {
      await execute(`yarn dashmate stop`).then( async res => {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(false, `${key} is running!`)
      }

      testnetConfig = await getConfig('testnet')

      await execute(`yarn dashmate status core`).then( async res => {
        if (res.status !== 1) {
          throw new Error(`Status core command should finish with exit code 1 instead of ${res.status}. \n${res.stderr}`)
        }
        expect(String(res.stderr)).to.include(`Error: Dash JSON-RPC: Request Error: connect ECONNREFUSED 127.0.0.1:${testnetConfig.get('core.rpc.port')}`)
      })

      await execute(`yarn dashmate status platform`).then( async res => {
        if (res.status !== 1) {
          throw new Error(`Status platform command should finish with exit code 1 instead of ${res.status}. \n${res.stderr}`)
        }
        expect(String(res.stderr)).to.include(`ServiceIsNotRunningError: Service ${SERVICES.drive_tenderdash} for testnet is not \n    running. Please run the service first.\n`)
      })

      await execute(`yarn dashmate status`).then( async res => {
        if (res.status !== 1) {
          throw new Error(`Status command should finish with exit code 1 instead of ${res.status}. \n${res.stderr}`)
        }
        expect(String(res.stderr)).to.include(`ServiceIsNotRunningError: Service ${SERVICES.core} for testnet is not running. Please \n    run the service first.\n`)
      })

      await execute(`yarn dashmate status masternode`).then( async res => {
        if (res.status !== 1) {
          throw new Error(`Status masternode command should finish with exit code 1 instead of ${res.status}. \n${res.stderr}`)
        }
        expect(String(res.stderr)).to.include(`Error: Dash JSON-RPC: Request Error: connect ECONNREFUSED 127.0.0.1:${testnetConfig.get('core.rpc.port')}`)
      })

      await execute(`yarn dashmate status services --format=json`).then( async res => {
        if (res.status !== undefined) {
          throw new Error(`Status services command should finish with exit code 1 instead of ${res.status}. \n${res.stderr}`)
        }
        const output = JSON.parse(res.toString())

        const services = ['Core', 'Sentinel', 'Drive ABCI', 'Drive Tenderdash', 'DAPI API', 'DAPI Transactions Filter Stream', 'DAPI Envoy']
        for (let serviceStatus of output) {
          expect(services).to.include(serviceStatus.Service);
          expect(serviceStatus.Status).to.equal('exited')
        }
      })
    });

    it('Start again testnet network', async () => {
      let output;

      await execute(`yarn dashmate start`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(true, `${key} is not running!`)
      }

      do {
        const status = await execute(`yarn dashmate status core --format=json`);
        output = JSON.parse(status.toString())
        await wait(5000);
      } while (+output['Header height'] < 1)
    });

    it('Reset testnet network', async () => {
      let output;

      await execute(`yarn dashmate reset`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(configEnvs, SERVICES[key]);
        expect(isRunning).to.equal(false, `${key} is running!`)
      }

      await execute(`yarn dashmate start`).then( res => {
        if(res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`)
        }
      })

      const resetTestnetConfig = getConfig(testnetNetwork);
      expect(isEqual(resetTestnetConfig, testnetConfig)).to.equal(true);

      for (const [key] of Object.entries(SERVICES)) {
        const isRunning = await container.isServiceRunning(resetTestnetConfig.toEnvs(), SERVICES[key]);
        expect(isRunning).to.equal(true, `${key} is not running!`)
      }

      do {
        const status = await execute(`yarn dashmate status core --format=json`);
        output = JSON.parse(status.toString())
        await wait(5000);
      } while (+output['Header height'] < 1)
    });
  });
});

async function getCoreService(config, docker) {
  return new CoreService(
    config,
    createRpcClient(
      {
        port: config.get('core.rpc.port'),
        user: config.get('core.rpc.user'),
        pass: config.get('core.rpc.password'),
      },
    ),
    docker,
  );
}

const fs = require('fs');
const { exec } = require('child_process');
const { promisify } = require('util');

const { Listr } = require('listr2');
const { Observable } = require('rxjs');

const { PrivateKey } = require('@dashevo/dashcore-lib');
const { NETWORK_LOCAL } = require('../../constants');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {waitForCorePeersConnected} waitForCorePeersConnected
 * @param {waitForMasternodesSync} waitForMasternodesSync
 * @param {createRpcClient} createRpcClient
 * @param {Docker} docker
 * @return {startNodeTask}
 */
function startNodeTaskFactory(
  dockerCompose,
  waitForCorePeersConnected,
  waitForMasternodesSync,
  createRpcClient,
  docker,
) {
  const execAsync = promisify(exec);
  const followDockerProgress = promisify(docker.modem.followProgress.bind(docker.modem));

  /**
   * @typedef {startNodeTask}
   * @param {Config} config
   * @param {Object} [options]
   * @param {boolean} [options.isMinerEnabled]
   * @return {Object}
   */
  function startNodeTask(
    config,
    {
      isMinerEnabled = undefined,
    } = {},
  ) {
    // Check external IP is set
    config.get('externalIp', true);

    if (isMinerEnabled === undefined) {
      // eslint-disable-next-line no-param-reassign
      isMinerEnabled = config.get('core.miner.enable');
    }

    if (isMinerEnabled === true && config.get('network') !== NETWORK_LOCAL) {
      throw new Error(`'core.miner.enabled' option only works with local network. Your network is ${config.get('network')}.`);
    }

    // Check Drive log files are created
    if (config.has('platform')) {
      const prettyFilePath = config.get('platform.drive.abci.log.prettyFile.path');

      if (!fs.existsSync(prettyFilePath)) {
        fs.writeFileSync(prettyFilePath, '');
      }

      const jsonFilePath = config.get('platform.drive.abci.log.jsonFile.path');

      if (!fs.existsSync(jsonFilePath)) {
        fs.writeFileSync(jsonFilePath, '');
      }
    }

    return new Listr([
      {
        title: 'Check node is not started',
        task: async () => {
          if (await dockerCompose.isServiceRunning(config.toEnvs())) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Build services',
        skip: (ctx) => ctx.skipFurtherServiceBuilds === true,
        enabled: () => config.has('platform')
          && (
            config.get('platform.dapi.api.docker.build.path') !== null
            || config.get('platform.drive.abci.docker.build.path') !== null
          ),
        task: (ctx) => {
          ctx.skipFurtherServiceBuilds = true;

          const serviceBuildConfigs = [
            {
              name: 'Drive',
              buildOptions: config.get('platform.drive.abci.docker.build'),
              serviceName: 'drive_abci',
            },
            {
              name: 'DAPI',
              buildOptions: config.get('platform.dapi.api.docker.build'),
              serviceName: 'dapi_api',
            },
          ];

          const buildTasks = serviceBuildConfigs
            .filter(({ buildOptions }) => buildOptions.path !== null)
            .map(({
              name,
              buildOptions,
              serviceName,
            }) => ({
              title: `Build ${name}`,
              task: () => (
                new Listr([
                  {
                    title: 'Build Docker image',
                    task: async () => {
                      const envs = config.toEnvs();

                      await dockerCompose.build(envs, serviceName);
                    },
                  },
                  {
                    title: 'Update NPM cache',
                    task: async () => {
                      // Build node_modules stage only to access to npm cache
                      const buildStream = await docker.buildImage({
                        context: buildOptions.path,
                        src: ['Dockerfile', 'docker/cache', 'package.json', 'package-lock.json'],
                      }, {
                        target: 'node_modules',
                      });

                      const output = await followDockerProgress(buildStream);

                      const buildError = output.find(({ error }) => error);

                      if (buildError) {
                        throw new Error(buildError.error);
                      }

                      const {
                        aux: {
                          ID: nodeModulesImageId,
                        },
                      } = output.find(({ aux }) => aux && aux.ID);

                      // Copy npm cache from node_modules stage image back to cache dir
                      const container = await docker.createContainer({
                        Image: nodeModulesImageId,
                      });

                      await Promise.all([
                        execAsync(`docker cp ${container.id}:/root/.cache ${buildOptions.path}/docker/cache`),
                        execAsync(`docker cp ${container.id}:/root/.npm ${buildOptions.path}/docker/cache`),
                      ]);

                      // Remove node_modules stage container and image
                      await container.remove();

                      const nodeModulesImage = docker.getImage(nodeModulesImageId);
                      await nodeModulesImage.remove();
                    },
                  },
                ])
              ),
            }));

          return new Listr(buildTasks);
        },
      },
      {
        title: 'Start services',
        task: async () => {
          const isMasternode = config.get('core.masternode.enable');
          if (isMasternode) {
            // Check operatorPrivateKey is set
            config.get('core.masternode.operator.privateKey', true);
          }

          const envs = config.toEnvs();

          await dockerCompose.up(envs);
        },
      },
      {
        title: 'Wait for peers to be connected',
        enabled: () => isMinerEnabled === true,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
          });

          await waitForCorePeersConnected(rpcClient);
        },
      },
      {
        title: 'Wait for sync',
        enabled: () => config.get('network') === NETWORK_LOCAL,
        task: async () => {
          const rpcClient = createRpcClient({
            port: config.get('core.rpc.port'),
            user: config.get('core.rpc.user'),
            pass: config.get('core.rpc.password'),
          });

          return new Observable(async (observer) => {
            await waitForMasternodesSync(
              rpcClient,
              (verificationProgress) => {
                observer.next(`${(verificationProgress * 100).toFixed(2)}% complete`);
              },
            );

            observer.complete();

            return this;
          });
        },
      },
      {
        title: 'Start a miner',
        enabled: () => isMinerEnabled === true,
        task: async () => {
          let minerAddress = config.get('core.miner.address');

          if (minerAddress === null) {
            const privateKey = new PrivateKey();
            minerAddress = privateKey.toAddress('regtest').toString();

            config.set('core.miner.address', minerAddress);
          }

          const minerInterval = config.get('core.miner.interval');

          await dockerCompose.execCommand(
            config.toEnvs(),
            'core',
            [
              'bash',
              '-c',
              `while true; do dash-cli generatetoaddress 1 ${minerAddress}; sleep ${minerInterval}; done`,
            ],
            ['--detach'],
          );
        },
      }]);
  }

  return startNodeTask;
}

module.exports = startNodeTaskFactory;

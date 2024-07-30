import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import chalk from 'chalk';
import {
  NETWORK_TESTNET,
} from '../../../../constants.js';

/**
 * @param {Config} config
 * @return {validateCoreDataDirectoryPath}
 */
function validateCoreDataDirectoryPathFactory(config) {
  /**
   * @typedef {function} validateCoreDataDirectoryPath
   * @param {string} value
   * @return {string|boolean}
   */
  function validateCoreDataDirectoryPath(value) {
    if (value.length === 0) {
      return 'should not be empty';
    }

    // Path must be absolute
    if (!path.isAbsolute(value)) {
      return 'path must be absolute';
    }

    // Should contain data and dashd.conf
    const configFilePath = path.join(value, 'dash.conf');

    let dataDirName = 'blocks';
    if (config.get('network') === NETWORK_TESTNET) {
      dataDirName = 'testnet3';
    }

    try {
      fs.accessSync(configFilePath, fs.constants.R_OK);
      fs.accessSync(path.join(value, dataDirName), fs.constants.R_OK);
    } catch (e) {
      return 'directory must be readable and contain blockchain data with dash.conf';
    }

    const configFileContent = fs.readFileSync(configFilePath, 'utf8');

    // Config file should contain testnet=1 in case of testnet
    // and shouldn't contain testnet=1, regtest=1 or devnet= in case of mainnet
    if (config.get('network') === NETWORK_TESTNET) {
      if (!configFileContent.includes('testnet=1')) {
        return 'dash.conf should be configured for testnet';
      }
    } else if (configFileContent.includes('testnet=1')
      || configFileContent.includes('regtest=1')
      || configFileContent.includes('devnet=')) {
      return 'dash.conf should be configured for mainnet';
    }

    return true;
  }

  return validateCoreDataDirectoryPath;
}

/**
 *
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @param {generateEnvs} generateEnvs
 * @return {importCoreDataTask}
 */
export default function importCoreDataTaskFactory(
  docker,
  dockerPull,
  generateEnvs,
) {
  /**
   * @typedef {function} importCoreDataTask
   * @returns {Listr}
   */
  async function importCoreDataTask() {
    return new Listr([
      {
        title: 'Import existing Core data',
        task: async (ctx, task) => {
          const doImport = await task.prompt({
            type: 'toggle',
            header: `  If you already run a masternode on this server, you can
   import your existing Dash Core data instead of syncing a new dashmate node from scratch.
   Your current user account must have read access to this directory.\n`,
            message: 'Import existing data?',
            enabled: 'Yes',
            disabled: 'No',
            initial: true,
          });

          if (!doImport) {
            task.skip();
            return;
          }

          // Masternode Operator key
          const coreDataPath = await task.prompt({
            type: 'input',
            header: `  Please enter path to your existing Dash Core data directory.

   - Your current user must have read access to this directory.
   - The data directory usually ends with .dashcore and contains dash.conf and the data files to import
   - If dash.conf is stored separately, you should copy or link to this file in the data directory\n`,
            message: 'Core data directory path',
            validate: validateCoreDataDirectoryPathFactory(ctx.config),
          });

          // Read configuration from dashd.conf
          const configPath = path.join(coreDataPath, 'dash.conf');
          const configFileContent = fs.readFileSync(configPath, 'utf8');

          // Config file should contain masternodeblsprivkey in case of masternode
          if (ctx.config.get('core.masternode.enable')) {
            const masternodeOperatorPrivateKey = configFileContent.match(/^masternodeblsprivkey=([^ \n]+)/m)?.[1];

            if (masternodeOperatorPrivateKey) {
              ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);
              // txindex is enabled by default for masternodes
              ctx.isReindexRequired = false;
            } else {
              // We need to reindex Core if there weren't all required indexed enabled before
              ctx.isReindexRequired = !configFileContent.match(/^txindex=1/);
            }
          }

          const host = configFileContent.match(/^bind=([^ \n]+)/m)?.[1];

          if (host) {
            ctx.config.set('core.p2p.host', host);
          }

          // Store values to fill in the configure node form

          // eslint-disable-next-line prefer-destructuring
          ctx.importedP2pPort = configFileContent.match(/^port=([^ \n]+)/m)?.[1];

          // eslint-disable-next-line prefer-destructuring
          ctx.importedExternalIp = configFileContent.match(/^externalip=([^ \n]+)/m)?.[1];

          // Copy data directory to docker a volume

          // Create a volume
          const { COMPOSE_PROJECT_NAME: composeProjectName } = generateEnvs(ctx.config);

          const volumeName = `${composeProjectName}_core_data`;

          // Check if volume already exists
          const volumes = await docker.listVolumes();
          const exists = volumes.Volumes.some((volume) => volume.Name === volumeName);

          if (exists) {
            throw new Error(`Volume ${volumeName} already exists. Please remove it first.`);
          }

          await docker.createVolume(volumeName);

          // Pull image
          await dockerPull('alpine');

          const commands = [
            `mkdir /${volumeName}/.dashcore/`,
            'cd /source',
            `cp -avL * /${volumeName}/.dashcore/`,
            `chown -R 1000:1000 /${volumeName}/`,
            `rm /${volumeName}/.dashcore/dash.conf`,
          ];

          // Copy data and set user dash
          const [result] = await docker.run(
            'alpine',
            [
              '/bin/sh',
              '-c',
              commands.join(' && '),
            ],
            task.stdout(),
            {
              HostConfig: {
                AutoRemove: true,
                Binds: [
                  `${coreDataPath}:/source:ro`,
                ],
                Mounts: [
                  {
                    Type: 'volume',
                    Source: volumeName,
                    Target: `/${volumeName}`,
                  },
                ],
              },
            },
          );

          if (result.StatusCode !== 0) {
            throw new Error('Cannot copy data dir to volume');
          }

          let header;
          if (ctx.isReindexRequired) {
            header = chalk`  {bold You existing Core node doesn't have indexes required to run ${ctx.nodeTypeName}.
  Reindex of the Core data will be needed after you finish the node setup.}

  Please stop your existing Dash Core node before reindexing.
  Also, disable any automatic startup services (e.g., cron, systemd) for the existing Dash Core installation.\n`;
          } else {
            header = `  Please stop your existing Dash Core node before starting the new dashmate-based
    node ("dashmate start"). Also, disable any automatic startup services (e.g., cron, systemd) for the existing Dash Core installation.\n`;
          }

          await task.prompt({
            type: 'confirm',
            header,
            message: 'Press any key to continue...',
            default: ' ',
            separator: () => '',
            format: () => '',
            initial: true,
            isTrue: () => true,
          });

          // eslint-disable-next-line no-param-reassign
          task.output = `${coreDataPath} imported`;
        },
        options: {
          persistentOutput: true,
        },
      },
    ]);
  }

  return importCoreDataTask;
}

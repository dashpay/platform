import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
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

    // Should contain dashd.conf
    const configFilePath = path.join(value, 'dash.conf');
    try {
      fs.accessSync(configFilePath, fs.constants.R_OK);
    } catch (e) {
      return 'directory must contain dash.conf and it must be readable by the current user';
    }

    let dataDirName = 'blocks';
    if (config.get('network') === NETWORK_TESTNET) {
      dataDirName = 'testnet3';
    }

    // Should contain data dir
    try {
      fs.accessSync(path.join(value, dataDirName), fs.constants.R_OK);
    } catch (e) {
      return 'directory must contain network data and it must be readable by the current user';
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

    // Config file should contain masternodeblsprivkey in case of masternode
    if (config.get('core.masternode.enable')) {
      if (!configFileContent.includes('masternodeblsprivkey=')) {
        return 'dash.conf should contain masternodeblsprivkey';
      }
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
export default function importCoreDataTaskFactory(docker, dockerPull, generateEnvs) {
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
            header: `  If you run a masternode on the same machine, you can
   import your existing data so you don't need to sync node from scratch.
   You current user must have read access to this directory.`,
            message: 'Try?',
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

You current user must have read access to this directory.\n`,
            message: 'Core data directory path',
            validate: validateCoreDataDirectoryPathFactory(ctx.config),
          });

          // Read configuration from dashd.conf
          const configPath = path.join(coreDataPath, 'dash.conf');
          const configFileContent = fs.readFileSync(configPath, 'utf8');
          const masternodeOperatorPrivateKey = configFileContent.match(/masternodeblsprivkey=(.*)/)[1];

          if (masternodeOperatorPrivateKey) {
            ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);
          }

          const host = configFileContent.match(/bind=(.*)/)[1];

          if (host) {
            ctx.config.set('core.p2p.host', host);
          }

          // Store values to fill in the configure node form

          // eslint-disable-next-line prefer-destructuring
          ctx.importedP2pPort = configFileContent.match(/port=(.*)/)[1];

          // eslint-disable-next-line prefer-destructuring
          ctx.importedExternalIp = configFileContent.match(/externalip=(.*)/)[1];

          // Copy data directory to docker a volume

          // Create a volume
          const { COMPOSE_PROJECT_NAME: composeProjectName } = generateEnvs(ctx.config);

          const volumeName = `${composeProjectName}_core_data`;
          await docker.createVolume(volumeName);

          // Pull image
          await dockerPull('alpine');

          // Copy data and set user dash
          const [result] = await docker.run(
            'alpine',
            [
              '/bin/sh',
              '-c',
              `cd /source && cp -av * /${volumeName}/ && chown -R 1000:1000 /${volumeName}/`,
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

          await task.prompt({
            type: 'confirm',
            header: `  You need to stop your existing node before your start a dashmate
    node\n`,
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

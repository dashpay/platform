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

          // Read masternode operator private key from dashd.conf
          const configPath = path.join(coreDataPath, 'dash.conf');
          const configFileContent = fs.readFileSync(configPath, 'utf8');
          const masternodeOperatorPrivateKey = configFileContent.match(/masternodeblsprivkey=(.*)/)[1];

          ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);

          // Copy data directory to docker a volume

          // Create a volume
          const { COMPOSE_PROJECT_NAME: composeProjectName } = generateEnvs(ctx.config);

          const volumeName = `${composeProjectName}_core_data`;
          await docker.createVolume(volumeName);

          // Pull image
          await dockerPull('alpine');

          const hostConfig = {
            AutoRemove: true,
            Binds: [
              `${coreDataPath}:/source:ro`,
            ],
            Mounts: [
              {
                Type: 'volume',
                Source: volumeName,
                Target: '/destination',
              },
            ],
          };

          // Copy data and set user dash
          const writableStream = new WritableStream();

          const [result] = await docker.run(
            'alpine',
            ['/bin/sh', '-c', 'cp -a /source/. /destination/ && chown -R 1000:1000 /destination/'],
            writableStream,
            {
              HostConfig: hostConfig,
            },
          );

          // TODO: Stream to output

          const output = writableStream.toString();

          if (result.StatusCode !== 0) {
            throw new Error(`Cannot copy data dir: ${output.substring(0, 100)}`);
          }

          await task.prompt({
            type: 'confirm',
            header: `  You need to stop your existing master before your start a dashmate
    node\n`,
            message: 'Yes I understand...',
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

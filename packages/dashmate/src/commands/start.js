const fs = require('fs').promises;
const path = require('path');

const Listr = require('listr');

const dotenv = require('dotenv');

const { flags: flagTypes } = require('@oclif/command');

const BaseCommand = require('../oclif/command/BaseCommand');

const UpdateRendererWithOutput = require('../oclif/renderer/UpdateRendererWithOutput');

const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

class StartCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
      'external-ip': externalIp,
      'core-p2p-port': coreP2pPort,
    },
    {
      'full-node': isFullNode,
      'operator-private-key': operatorPrivateKey,
      'drive-image-build-path': driveImageBuildPath,
      'dapi-image-build-path': dapiImageBuildPath,
    },
    dockerCompose,
  ) {
    const tasks = new Listr([
      {
        title: `Start ${isFullNode ? 'full node' : 'masternode'} with ${preset} preset`,
        task: async () => {
          let CORE_MASTERNODE_BLS_PRIV_KEY;

          if (operatorPrivateKey) {
            CORE_MASTERNODE_BLS_PRIV_KEY = operatorPrivateKey;
          }

          if (isFullNode) {
            CORE_MASTERNODE_BLS_PRIV_KEY = '';
          }

          const envs = {
            CORE_MASTERNODE_BLS_PRIV_KEY,
            CORE_P2P_PORT: coreP2pPort,
            CORE_EXTERNAL_IP: externalIp,
            DRIVE_IMAGE_BUILD_PATH: driveImageBuildPath,
            DAPI_IMAGE_BUILD_PATH: dapiImageBuildPath,
          };

          if (driveImageBuildPath || dapiImageBuildPath) {
            if (preset === 'testnet') {
              throw new Error('You can\' use drive-image-build-path and dapi-image-build-path options with testnet preset');
            }

            const envFile = path.join(__dirname, '..', '..', `.env.${preset}`);
            const envRawData = await fs.readFile(envFile);
            let { COMPOSE_FILE: composeFile } = dotenv.parse(envRawData);

            if (driveImageBuildPath) {
              composeFile = `${composeFile}:docker-compose.platform.build-drive.yml`;
            }

            if (dapiImageBuildPath) {
              composeFile = `${composeFile}:docker-compose.platform.build-dapi.yml`;
            }

            envs.COMPOSE_FILE = composeFile;
          }

          await dockerCompose.up(preset, envs);
        },
      },
    ],
    { collapse: false, renderer: UpdateRendererWithOutput });

    try {
      await tasks.run();
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

StartCommand.description = `Start masternode
...
Start masternode with specific preset
`;

StartCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}, {
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'core-p2p-port',
  required: true,
  description: 'Core P2P port',
}];

StartCommand.flags = {
  'full-node': flagTypes.boolean({ char: 'f', description: 'start as full node', default: false }),
  'operator-private-key': flagTypes.string({ char: 'p', description: 'operator private key', default: null }),
  'drive-image-build-path': flagTypes.string({ description: 'drive\'s docker image build path', default: null }),
  'dapi-image-build-path': flagTypes.string({ description: 'dapi\'s docker image build path', default: null }),
};

module.exports = StartCommand;

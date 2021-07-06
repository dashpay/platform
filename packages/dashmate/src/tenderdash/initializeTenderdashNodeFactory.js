const { WritableStream } = require('memory-streams');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @return {initializeTenderdashNode}
 */
function initializeTenderdashNodeFactory(dockerCompose, docker, dockerPull) {
  /**
   * @typedef {initializeTenderdashNode}
   * @param {Config} config
   * @return {Promise<Object>}
   */
  async function initializeTenderdashNode(config) {
    if (await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash')) {
      throw new Error('Can\'t initialize Tenderdash. Already running.');
    }

    const { COMPOSE_PROJECT_NAME: composeProjectName } = config.toEnvs();
    const volumeName = 'drive_tenderdash';
    const volumeNameFullName = `${composeProjectName}_${volumeName}`;

    const volume = docker.getVolume(volumeNameFullName);

    const isVolumeDefined = await volume.inspect()
      .then(() => true)
      .catch(() => false);

    if (!isVolumeDefined) {
      // Create volume with tenderdash data
      await docker.createVolume({
        Name: volumeNameFullName,
        Labels: {
          'com.docker.compose.project': composeProjectName,
          'com.docker.compose.version': '1.27.4',
          'com.docker.compose.volume': volumeName,
        },
      });
    }

    // Initialize Tenderdash

    const tenderdashImage = config.get('platform.drive.tenderdash.docker.image', true);

    await dockerPull(tenderdashImage);

    const writableStream = new WritableStream();

    const command = [
      '/usr/bin/tenderdash init > /dev/null',
      'echo "["',
      'cat $TMHOME/config/node_key.json',
      'echo ","',
      'cat $TMHOME/config/genesis.json',
      'echo ",\\""',
      '/usr/bin/tenderdash show-node-id',
      'echo "\\""',
      'echo "]"',
      'rm -rf $TMHOME/config',
    ].join('&&');

    const [result] = await docker.run(
      tenderdashImage,
      [],
      writableStream,
      {
        Entrypoint: ['sh', '-c', command],
        HostConfig: {
          AutoRemove: true,
          Binds: [`${volumeNameFullName}:/tenderdash`],
        },
      },
    );

    if (result.StatusCode !== 0) {
      let message = writableStream.toString();

      if (result.StatusCode === 1 && message === '') {
        message = 'already initialized. Please reset node data';
      }

      throw new Error(`Can't initialize tenderdash: ${message}`);
    }

    let stringifiedJSON = writableStream.toString();
    stringifiedJSON = stringifiedJSON.replace(/\r\n/g, '');

    return JSON.parse(stringifiedJSON);
  }

  return initializeTenderdashNode;
}

module.exports = initializeTenderdashNodeFactory;

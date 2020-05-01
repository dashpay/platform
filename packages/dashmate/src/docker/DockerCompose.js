const path = require('path');

const dockerCompose = require('docker-compose');

const hasbin = require('hasbin');
const semver = require('semver');

const DockerComposeError = require('./errors/DockerComposeError');
const ServiceAlreadyRunningError = require('./errors/ServiceAlreadyRunningError');

class DockerCompose {
  /**
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   */
  constructor(docker, startedContainers) {
    this.docker = docker;
    this.startedContainers = startedContainers;
  }

  /**
   * Run service
   *
   * @param {string} preset
   * @param {string} serviceName
   * @param {array} [command]
   * @param {array} [options]
   * @return {Promise<Container>}
   */
  async runService(preset, serviceName, command = [], options = []) {
    if (await this.isServiceRunning(preset, serviceName)) {
      throw new ServiceAlreadyRunningError(preset, serviceName);
    }

    let containerName;
    const env = this.getPlaceholderEmptyEnvOptions();

    try {
      ({ out: containerName } = await dockerCompose.run(
        serviceName,
        command,
        {
          ...this.getOptions(preset, env),
          commandOptions: options,
        },
      ));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    containerName = containerName.trim();

    this.startedContainers.addContainer(containerName);
    return this.docker.getContainer(containerName);
  }

  /**
   * Is service running?
   *
   * @param {string} preset
   * @param {string} [serviceName]
   * @return {Promise<boolean>}
   */
  async isServiceRunning(preset, serviceName = undefined) {
    await this.throwErrorIfNotInstalled();

    let psOutput;

    const env = this.getPlaceholderEmptyEnvOptions();

    try {
      ({ out: psOutput } = await dockerCompose.ps({
        ...this.getOptions(preset, env),
        commandOptions: ['-q', serviceName],
      }));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    const coreContainerIds = psOutput.trim()
      .split('\n')
      .filter((containerId) => containerId !== '');

    for (const containerId of coreContainerIds) {
      const container = this.docker.getContainer(containerId);

      const { State: { Status: status } } = await container.inspect();

      if (status === 'running') {
        return true;
      }
    }

    return false;
  }

  /**
   * Down docker compose
   *
   * @param {string} preset
   * @return {Promise<void>}
   */
  async down(preset) {
    await this.throwErrorIfNotInstalled();

    const env = this.getPlaceholderEmptyEnvOptions();

    try {
      await dockerCompose.down({
        ...this.getOptions(preset, env),
        commandOptions: ['-v'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * @private
   * @return {Promise<void>}
   */
  async throwErrorIfNotInstalled() {
    if (!hasbin.sync('docker')) {
      throw new Error('Docker is not installed');
    }

    if (!hasbin.sync('docker-compose')) {
      throw new Error('Docker Compose is not installed');
    }

    const { out: version } = await dockerCompose.version();
    if (semver.lt(version.trim(), DockerCompose.DOCKER_COMPOSE_MIN_VERSION)) {
      throw new Error(`Update Docker Compose to version ${DockerCompose.DOCKER_COMPOSE_MIN_VERSION} or higher`);
    }
  }

  /**
   * @private
   * @param {string} preset
   * @param {Object} [envOptions]
   * @return {{cwd: string, config: string, composeOptions: [string, string]}}
   */
  getOptions(preset, envOptions = undefined) {
    let env;

    if (envOptions !== undefined) {
      env = Object.assign(process.env, envOptions);
    }

    return {
      cwd: path.join(__dirname, '../../'),
      composeOptions: [
        '--env-file', `.env.${preset}`,
      ],
      env,
    };
  }

  /**
   * @private
   * @return {Object}
   */
  getPlaceholderEmptyEnvOptions() {
    return {
      CORE_EXTERNAL_IP: '127.0.0.1',
    };
  }
}

DockerCompose.DOCKER_COMPOSE_MIN_VERSION = '1.25.0';

module.exports = DockerCompose;

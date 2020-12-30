const path = require('path');

const dockerCompose = require('docker-compose');

const hasbin = require('hasbin');
const semver = require('semver');

const DockerComposeError = require('./errors/DockerComposeError');
const ServiceAlreadyRunningError = require('./errors/ServiceAlreadyRunningError');
const ServiceIsNotRunningError = require('./errors/ServiceIsNotRunningError');
const ContainerIsNotPresentError = require('./errors/ContainerIsNotPresentError');

class DockerCompose {
  /**
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   * @param {string} homeDirPath
   */
  constructor(docker, startedContainers, homeDirPath) {
    this.docker = docker;
    this.startedContainers = startedContainers;
    this.homeDirPath = homeDirPath;
  }

  /**
   * Run service
   *
   * @param {Object} envs
   * @param {string} serviceName
   * @param {array} [command]
   * @param {array} [options]
   * @return {Promise<Container>}
   */
  async runService(envs, serviceName, command = [], options = []) {
    if (await this.isServiceRunning(envs, serviceName)) {
      throw new ServiceAlreadyRunningError(serviceName);
    }

    let containerName;

    try {
      ({ out: containerName } = await dockerCompose.run(
        serviceName,
        command,
        {
          ...this.getOptions(envs),
          commandOptions: options,
        },
      ));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    containerName = containerName.trim().split('\n').pop();

    this.startedContainers.addContainer(containerName);

    return this.docker.getContainer(containerName);
  }

  /**
   * Is service running?
   *
   * @param {Object} envs
   * @param {string} [serviceName]
   * @return {Promise<boolean>}
   */
  async isServiceRunning(envs, serviceName = undefined) {
    await this.throwErrorIfNotInstalled();

    const coreContainerIds = await this.getContainersList(envs, serviceName);

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
   * Up docker compose
   *
   * @param {Object} envs
   * @return {Promise<void>}
   */
  async up(envs) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.upAll({
        ...this.getOptions(envs),
        commandOptions: ['--build'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Stop all docker compose containers
   *
   * @param {Object} envs
   * @return {Promise<void>}
   */
  async stop(envs) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.stop(this.getOptions(envs));
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Inspect service
   *
   * @param {Object} envs
   * @param {string} serviceName
   * @return {Promise<object>}
   */
  async inspectService(envs, serviceName) {
    await this.throwErrorIfNotInstalled();

    const containerIds = await this.getContainersList(envs, serviceName);

    if (containerIds.length === 0) {
      throw new ContainerIsNotPresentError(serviceName);
    }

    const container = this.docker.getContainer(containerIds[0]);

    return container.inspect();
  }

  /**
   * Execute command
   *
   * @param {Object} envs
   * @param {string} serviceName
   * @param {string} command
   * @param {string[]} [commandOptions]
   * @return {Promise<object>}
   */
  async execCommand(envs, serviceName, command, commandOptions = []) {
    await this.throwErrorIfNotInstalled();

    if (!(await this.isServiceRunning(envs, serviceName))) {
      throw new ServiceIsNotRunningError(envs.CONFIG_NAME, serviceName);
    }

    let commandOutput;

    const options = {
      ...this.getOptions(envs),
      commandOptions,
    };

    try {
      commandOutput = await dockerCompose.exec(
        serviceName,
        command,
        options,
      );
    } catch (e) {
      throw new DockerComposeError(e);
    }

    return commandOutput;
  }

  /**
   * Get list of Docker containers
   *
   * @param {Object} envs
   * @param {string} [filterServiceName]
   * @return {string[]}
   */
  async getContainersList(envs, filterServiceName = undefined) {
    let psOutput;

    try {
      ({ out: psOutput } = await dockerCompose.ps({
        ...this.getOptions(envs),
        commandOptions: ['-q', filterServiceName],
      }));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    return psOutput
      .trim()
      .split('\n')
      .filter(Boolean);
  }

  /**
   * Down docker compose
   *
   * @param {Object} envs
   * @return {Promise<void>}
   */
  async down(envs) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.down({
        ...this.getOptions(envs),
        commandOptions: ['-v', '--remove-orphans'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Pull docker compose
   *
   * @param {Object} envs
   * @return {Promise<void>}
   */
  async pull(envs) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.pullAll({
        ...this.getOptions(envs),
        commandOptions: ['-q'],
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
   * @param {Object} envs
   * @return {{cwd: string, env: Object}}
   */
  getOptions(envs) {
    const env = {
      ...process.env,
      ...envs,
      MN_HOME_DIR: this.homeDirPath,
    };

    return {
      cwd: path.join(__dirname, '..', '..'),
      env,
    };
  }
}

DockerCompose.DOCKER_COMPOSE_MIN_VERSION = '1.25.0';

module.exports = DockerCompose;

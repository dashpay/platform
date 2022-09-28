const path = require('path');

const dockerCompose = require('@dashevo/docker-compose');

const hasbin = require('hasbin');
const semver = require('semver');

const { exec } = require('child_process');

const DockerComposeError = require('./errors/DockerComposeError');
const ServiceAlreadyRunningError = require('./errors/ServiceAlreadyRunningError');
const ServiceIsNotRunningError = require('./errors/ServiceIsNotRunningError');
const ContainerIsNotPresentError = require('./errors/ContainerIsNotPresentError');

const { HOME_DIR_PATH } = require('../constants');

class DockerCompose {
  /**
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   */
  constructor(docker, startedContainers) {
    this.docker = docker;
    this.startedContainers = startedContainers;
    this.isDockerSetupVerified = false;
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
    await this.throwErrorIfNotInstalled();

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

    containerName = containerName.trim().split(/\r?\n/).pop();

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

      let status;

      try {
        ({ State: { Status: status } } = await container.inspect());
      } catch (e) {
        if (!e.message.includes(`No such container: ${containerId}`)) {
          throw e;
        }
      }

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
        commandOptions: ['--no-build'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Build docker compose images
   *
   * @param {Object} envs
   * @param {string} [serviceName]
   * @return {Promise<ChildProcess>}
   */
  // eslint-disable-next-line no-unused-vars
  async build(envs, serviceName = undefined) {
    await this.throwErrorIfNotInstalled();

    try {
      // Temporarily build with buildx bake until docker compose build selects correct builder
      // https://github.com/docker/compose-cli/issues/1840
      const childProcess = exec(
        'docker buildx bake --progress plain --load -f docker-compose.platform.build.yml',
        this.getOptions(envs),
      );

      childProcess.isReady = new Promise((resolve, reject) => {
        childProcess.on('exit', (code) => {
          if (code === 0) {
            resolve(childProcess);
          } else {
            reject(childProcess);
          }
        });
      });

      return childProcess;
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
   * @param {string} [filterServiceNames]
   * @param {boolean} returnServiceNames
   * @return {string[]}
   */
  async getContainersList(
    envs,
    filterServiceNames = undefined,
    returnServiceNames = false,
  ) {
    let psOutput;
    const commandOptions = [];

    if (returnServiceNames) {
      commandOptions.push('--services');
    } else {
      commandOptions.push('--quiet');
    }

    commandOptions.push(filterServiceNames);

    try {
      ({ out: psOutput } = await dockerCompose.ps({
        ...this.getOptions(envs),
        commandOptions,
      }));
    } catch (e) {
      if (e.err && e.err.startsWith('no such service:')) {
        return [];
      }

      throw new DockerComposeError(e);
    }

    return psOutput
      .trim()
      .split(/\r?\n/)
      .filter(Boolean);
  }

  /**
   * Get list of Docker volumes
   * @param {Object} envs
   * @return {Promise<string[]>}
   */
  async getVolumeNames(envs) {
    let volumeOutput;
    try {
      ({ out: volumeOutput } = await dockerCompose.configVolumes({
        ...this.getOptions(envs),
      }));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    return volumeOutput
      .trim()
      .split(/\r?\n/);
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
   * Remove docker compose
   *
   * @param {Object} envs
   * @param {string[]} [serviceNames]
   * @return {Promise<void>}
   */
  async rm(envs, serviceNames) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.rm({
        ...this.getOptions(envs),
        commandOptions: ['--stop', '-v'],
      }, ...serviceNames);
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
    if (this.isDockerSetupVerified) {
      return;
    }

    this.isDockerSetupVerified = true;

    // Check docker
    if (!hasbin.sync('docker')) {
      throw new Error('Docker is not installed');
    }

    const dockerVersion = await new Promise((resolve, reject) => {
      this.docker.version((err, data) => {
        if (err) {
          return reject(err);
        }

        return resolve(data.Version);
      });
    });

    if (semver.lt(dockerVersion.trim(), DockerCompose.DOCKER_MIN_VERSION)) {
      throw new Error(`Update Docker to version ${DockerCompose.DOCKER_MIN_VERSION} or higher`);
    }

    let version;

    // Check docker compose
    try {
      ({ out: version } = await dockerCompose.version());
    } catch (e) {
      throw new Error('Docker Compose V2 is not available in your system');
    }

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
      DASHMATE_HOME_DIR: HOME_DIR_PATH,
    };

    return {
      cwd: path.join(__dirname, '..', '..'),
      env,
    };
  }
}

DockerCompose.DOCKER_COMPOSE_MIN_VERSION = '2.0.0';
DockerCompose.DOCKER_MIN_VERSION = '20.10.0';

module.exports = DockerCompose;

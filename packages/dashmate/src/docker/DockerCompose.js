const { Observable } = require('rxjs');

const isWsl = require('is-wsl');

const dockerCompose = require('@dashevo/docker-compose');

const hasbin = require('hasbin');
const semver = require('semver');

const DockerComposeError = require('./errors/DockerComposeError');
const ServiceAlreadyRunningError = require('./errors/ServiceAlreadyRunningError');
const ServiceIsNotRunningError = require('./errors/ServiceIsNotRunningError');
const ContainerIsNotPresentError = require('./errors/ContainerIsNotPresentError');

const { PACKAGE_ROOT_DIR } = require('../constants');

class DockerCompose {
  /**
   * Minimal
   *
   * @type {string}
   */
  static DOCKER_COMPOSE_MIN_VERSION = '2.0.0';

  /**
   * @type {string}
   */
  static DOCKER_MIN_VERSION = '20.10.0';

  /**
   * @type {Docker}
   */
  #docker;

  /**
   * @type {StartedContainers}
   */
  #startedContainers;

  /**
   * @type {boolean}
   */
  #isDockerSetupVerified = false;

  /**
   * @type {HomeDir}
   */
  #homeDir;

  /**
   * @type {function}
   */
  #generateEnvs;

  /**
   * @type {function}
   */
  #getServiceList;

  /**
   * @param {Docker} docker
   * @param {StartedContainers} startedContainers
   * @param {HomeDir} homeDir
   * @param {generateEnvs} generateEnvs
   * @param {getServiceList} getServiceList
   */
  constructor(docker, startedContainers, homeDir, generateEnvs, getServiceList) {
    this.#docker = docker;
    this.#startedContainers = startedContainers;
    this.#homeDir = homeDir;
    this.#generateEnvs = generateEnvs;
    this.#getServiceList = getServiceList;
  }

  /**
   * Run service
   *
   * @param {Config} config
   * @param {string} serviceName
   * @param {array} [command]
   * @param {array} [options]
   * @return {Promise<Container>}
   */
  async runService(config, serviceName, command = [], options = []) {
    await this.throwErrorIfNotInstalled();

    if (await this.isServiceRunning(config, serviceName)) {
      throw new ServiceAlreadyRunningError(serviceName);
    }

    let containerName;

    try {
      ({ out: containerName } = await dockerCompose.run(
        serviceName,
        command,
        {
          ...this.#createOptions(config),
          commandOptions: options,
        },
      ));
    } catch (e) {
      throw new DockerComposeError(e);
    }

    containerName = containerName.trim().split(/\r?\n/).pop();

    this.#startedContainers.addContainer(containerName);

    return this.#docker.getContainer(containerName);
  }

  /**
   * Checks if node is running by checking whether first container
   * from the targeted node is in `running` state
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.profiles] - Filter by profiles
   * @return {Promise<boolean>}
   */
  async isNodeRunning(config, options) {
    await this.throwErrorIfNotInstalled();

    const serviceList = this.#getServiceList(config);

    const filterServiceNames = serviceList.map((service) => service.name);

    const serviceContainers = await this.getContainersList(config, {
      formatJson: true,
      filterServiceNames,
      ...options,
    });

    for (const { State: state } of serviceContainers) {
      if (state === 'running') {
        return true;
      }
    }

    return false;
  }

  /**
   * Checks if service is running
   *
   * @param {Config} config
   * @param {string} serviceName filter by service name
   * @return {Promise<boolean>}
   */
  async isServiceRunning(config, serviceName) {
    await this.throwErrorIfNotInstalled();

    const [container] = await this.getContainersList(config, {
      filterServiceNames: serviceName,
      formatJson: true,
    });

    return container?.State === 'running';
  }

  /**
   * Up docker compose
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.profiles] - Filter by profiles
   * @return {Promise<void>}
   */
  async up(config, options) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.upAll({
        ...this.#createOptions(config, options),
        commandOptions: ['--no-build'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Build docker compose images
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string} [options.serviceName]
   * @return {Observable<{string}>}
   */
  // eslint-disable-next-line no-unused-vars
  async build(config, options = {}) {
    const envs = this.#generateEnvs(config);

    return this.buildWithEnvs(envs, options);
  }

  /**
   * Build docker compose images
   *
   * @param {Object} envs
   * @param {Object} [options]
   * @param {string} [options.serviceName]
   * @return {Observable<{string}>}
   */
  // eslint-disable-next-line no-unused-vars
  async buildWithEnvs(envs, options = {}) {
    try {
      return new Observable(async (observer) => {
        await this.throwErrorIfNotInstalled();

        const callback = (e) => {
          observer.next(e.toString());
        };

        if (options.serviceName) {
          await dockerCompose.buildOne(options.serviceName, {
            ...this.#createOptionsWithEnvs(envs),
            callback,
          });
        } else {
          await dockerCompose.buildAll({
            ...this.#createOptionsWithEnvs(envs),
            callback,
          });
        }

        observer.complete();
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Stop all docker compose containers
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.profiles] - Filter by profiles
   * @return {Promise<void>}
   */
  async stop(config, options = {}) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.stop(this.#createOptions(config, options));
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Inspect service
   *
   * @param {Config} config
   * @param {string} serviceName
   * @return {Promise<object>}
   */
  async inspectService(config, serviceName) {
    await this.throwErrorIfNotInstalled();

    const containerIds = await this.getContainersList(config, {
      filterServiceNames: serviceName,
      quiet: true,
    });

    if (containerIds.length === 0) {
      throw new ContainerIsNotPresentError(serviceName);
    }

    const container = this.#docker.getContainer(containerIds[0]);

    return container.inspect();
  }

  /**
   * Execute command
   *
   * @param {Config} config
   * @param {string} serviceName
   * @param {string} command
   * @param {string[]} [commandOptions]
   * @return {Promise<object>}
   */
  async execCommand(config, serviceName, command, commandOptions = []) {
    await this.throwErrorIfNotInstalled();

    if (!(await this.isServiceRunning(config, serviceName))) {
      throw new ServiceIsNotRunningError(config.getName(), serviceName);
    }

    let commandOutput;

    const options = {
      ...this.#createOptions(config),
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
   * @param {Config} config
   * @param {Object} [options={}] optional
   * @param {string|string[]} [options.filterServiceNames=false] - Filter by service name
   * @param {boolean} [options.returnServiceNames] - Return only service names
   * @param {boolean} [options.quiet=false] - Return only container ids
   * @param {boolean} [options.formatJson=false] - Return as json with details
   * @param {boolean} [options.all=false] - Return stopped containers as well
   * @return {Promise<string[]|object[]>}
   */
  async getContainersList(
    config,
    {
      filterServiceNames = undefined,
      returnServiceNames = false,
      quiet = false,
      formatJson = false,
      all = false,
      profiles = [],
    } = {},
  ) {
    let psOutput;
    const commandOptions = [];

    if (returnServiceNames) {
      commandOptions.push('--services');
    }

    if (quiet) {
      commandOptions.push('--quiet');
    }

    if (formatJson) {
      commandOptions.push('--format', 'json');
    }

    if (all) {
      commandOptions.push('--all');
    }

    commandOptions.push(filterServiceNames);

    try {
      ({ out: psOutput } = await dockerCompose.ps({
        ...this.#createOptions(config, { profiles }),
        commandOptions,
      }));
    } catch (e) {
      if (e.err && e.err.startsWith('no such service:')) {
        return [];
      }

      throw new DockerComposeError(e);
    }

    const containerList = psOutput
      .trim()
      .split(/\r?\n/)
      .filter(Boolean);

    if (containerList.length > 0 && formatJson) {
      return JSON.parse(containerList[0]);
    }

    return containerList;
  }

  /**
   * Get list of Docker volumes
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.profiles] - Filter by profiles
   * @return {Promise<string[]>}
   */
  async getVolumeNames(config, options = {}) {
    let volumeOutput;

    try {
      ({ out: volumeOutput } = await dockerCompose.configVolumes({
        ...this.#createOptions(config, options),
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
   * @param {Config} config
   * @return {Promise<void>}
   */
  async down(config) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.down({
        ...this.#createOptions(config),
        commandOptions: ['-v', '--remove-orphans'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Remove docker compose
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.serviceNames]
   * @param {string[]} [options.profiles]
   * @return {Promise<void>}
   */
  async rm(config, { serviceNames = [], profiles = [] } = {}) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.rm({
        ...this.#createOptions(config, { profiles }),
        commandOptions: ['--stop'],
      }, ...serviceNames);
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Pull docker compose
   *
   * @param {Config} config
   * @return {Promise<void>}
   */
  async pull(config) {
    await this.throwErrorIfNotInstalled();

    try {
      await dockerCompose.pullAll({
        ...this.#createOptions(config),
        commandOptions: ['-q'],
      });
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * @return {Promise<void>}
   */
  async throwErrorIfNotInstalled() {
    if (this.#isDockerSetupVerified) {
      return;
    }

    this.#isDockerSetupVerified = true;

    const dockerComposeInstallLink = 'https://docs.docker.com/compose/install/';
    const dockerInstallLink = 'https://docs.docker.com/engine/install/';
    const dockerPostInstallLinuxLink = 'https://docs.docker.com/engine/install/linux-postinstall/';
    const dockerContextLink = 'https://docs.docker.com/engine/context/working-with-contexts/';

    // Check docker
    if (!hasbin.sync('docker')) {
      throw new Error(`Docker is not installed. Please follow instructions ${dockerInstallLink}`);
    }

    let dockerVersion;
    try {
      dockerVersion = await new Promise((resolve, reject) => {
        this.#docker.version((err, data) => {
          if (err) {
            return reject(err);
          }

          return resolve(data.Version);
        });
      });
    } catch (e) {
      throw new Error(`Can't connect to Docker Engine: ${e.message}.\n\nPossible reasons:\n1. Docker is not started\n2. Permission issues ${dockerPostInstallLinuxLink}\n3. Wrong context ${dockerContextLink}`);
    }

    if (semver.lt(dockerVersion.trim(), DockerCompose.DOCKER_MIN_VERSION)) {
      throw new Error(`Update Docker to version ${DockerCompose.DOCKER_MIN_VERSION} or higher. Please follow instructions ${dockerInstallLink}`);
    }

    let version;

    // Check docker compose
    try {
      ({ out: version } = await dockerCompose.version());
    } catch (e) {
      throw new Error(`Docker Compose V2 is not available in your system. Please follow instructions ${dockerComposeInstallLink}`);
    }

    if (semver.lt(version.trim(), DockerCompose.DOCKER_COMPOSE_MIN_VERSION)) {
      throw new Error(`Update Docker Compose to version ${DockerCompose.DOCKER_COMPOSE_MIN_VERSION} or higher. Please follow instructions ${dockerComposeInstallLink}`);
    }
  }

  /**
   * @private
   * @param {Config} config
   * @param {Object} [options]
   * @return {{cwd: string, env: Object}}
   */
  #createOptions(config, options = {}) {
    const envs = this.#generateEnvs(config);

    return this.#createOptionsWithEnvs(envs, options);
  }

  /**
   * @private
   * @param {Object} envs
   * @param {Object} [options]
   * @return {{cwd: string, env: Object}}
   */
  #createOptionsWithEnvs(envs, options = {}) {
    const env = {
      ...process.env,
      ...envs,
    };

    if (isWsl) {
      // Solving issue under WSL when after restart container volume is not being mounted properly
      // https://github.com/docker/for-win/issues/4812
      // Following fix forces container recreation
      env.WSL2_FIX = Date.now();
    }

    const composeOptions = [];

    if (options.profiles?.length > 0) {
      options.profiles.forEach((profile) => {
        composeOptions.push('--profile', profile);
      });
    }

    return {
      cwd: PACKAGE_ROOT_DIR,
      env,
      composeOptions,
    };
  }
}

module.exports = DockerCompose;

import { Observable } from 'rxjs';

import isWsl from 'is-wsl';
import dockerCompose from '@dashevo/docker-compose';

import hasbin from 'hasbin';
import semver from 'semver';
import util from 'node:util';

import { PACKAGE_ROOT_DIR } from '../constants.js';
import ServiceAlreadyRunningError from './errors/ServiceAlreadyRunningError.js';
import DockerComposeError from './errors/DockerComposeError.js';
import ServiceIsNotRunningError from './errors/ServiceIsNotRunningError.js';
import ContainerIsNotPresentError from './errors/ContainerIsNotPresentError.js';

export default class DockerCompose {
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
      const { out } = await dockerCompose.run(
        serviceName,
        command,
        {
          ...this.#createOptions(config),
          commandOptions: options,
        },
      );

      containerName = out.trim();
    } catch (e) {
      throw new DockerComposeError(e);
    }

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
  async isNodeRunning(config, options = { profiles: [] }) {
    await this.throwErrorIfNotInstalled();

    let serviceList = this.#getServiceList(config, options);

    if (options.profiles?.length > 0) {
      serviceList = serviceList.filter((service) => (
        service.profiles.some((profile) => options.profiles.includes(profile))
      ));
    }

    const filterServiceNames = serviceList.map((service) => service.name);

    const serviceContainers = await this.getContainersList(config, {
      filterServiceNames,
    });

    return serviceContainers
      .find((container) => container.State === 'running') !== undefined;
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
   * @return {Promise<Object>}
   */
  async inspectService(config, serviceName) {
    await this.throwErrorIfNotInstalled();

    const containerIds = await this.getContainerIds(config, {
      filterServiceNames: serviceName,
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
   * Get compose container ids
   *
   * @param {Config} config
   * @param {Object} [options={}] optional
   * @param {string|string[]} [options.filterServiceNames] - Filter by service name
   * @param {boolean} [options.all=false] - Return stopped containers as well
   * @return {Promise<string[]>}
   */
  async getContainerIds(
    config,
    {
      filterServiceNames = undefined,
      all = false,
    } = {},
  ) {
    const commandOptions = ['--quiet'];

    if (all) {
      commandOptions.push('--all');
    }

    if (filterServiceNames) {
      commandOptions.push(filterServiceNames);
    }

    try {
      const { data: { services } } = await dockerCompose.ps({
        ...this.#createOptions(config),
        commandOptions,
      });

      return services.map((service) => service.name);
    } catch (e) {
      if (e.err && e.err.startsWith('no such service:')) {
        return [];
      }

      throw new DockerComposeError(e);
    }
  }

  /**
   * Get list of compose containers
   *
   * @param {Config} config
   * @param {Object} [options={}] optional
   * @param {string|string[]} [options.filterServiceNames] - Filter by service name
   * @param {boolean} [options.all=false] - Return stopped containers as well
   * @return {Promise<object[]>}
   */
  async getContainersList(
    config,
    {
      filterServiceNames = undefined,
      all = false,
    } = {},
  ) {
    const commandOptions = ['--format', 'json'];

    if (all) {
      commandOptions.push('--all');
    }

    if (filterServiceNames) {
      commandOptions.push(filterServiceNames);
    }

    try {
      const { data: { json } } = await dockerCompose.ps({
        ...this.#createOptions(config),
        commandOptions,
      });

      return json;
    } catch (e) {
      if (e.err && e.err.startsWith('no such service:')) {
        return [];
      }

      throw new DockerComposeError(e);
    }
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
    try {
      const { data: { volumes } } = await dockerCompose.configVolumes({
        ...this.#createOptions(config, options),
      });

      return volumes;
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * Down docker compose
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {Object} [options.removeVolumes=false]
   * @return {Promise<void>}
   */
  async down(config, options = {}) {
    await this.throwErrorIfNotInstalled();

    const commandOptions = ['--remove-orphans'];

    if (options.removeVolumes) {
      commandOptions.push('-v');
    }

    try {
      await dockerCompose.down({
        ...this.#createOptions(config),
        commandOptions,
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

    await this.throwErrorIfDockerIsNotInstalled();

    await this.throwErrorIfDockerComposeIsNotInstalled();
  }

  /**
   * @private
   */
  async throwErrorIfDockerIsNotInstalled() {
    const dockerInstallLink = 'https://docs.docker.com/engine/install/';
    const dockerPostInstallLinuxLink = 'https://docs.docker.com/engine/install/linux-postinstall/';
    const dockerContextLink = 'https://docs.docker.com/engine/context/working-with-contexts/';

    // Check docker
    if (!hasbin.sync('docker')) {
      throw new Error(`Docker is not installed. Please follow instructions ${dockerInstallLink}`);
    }

    let dockerVersionInfo;
    try {
      dockerVersionInfo = await this.#docker.version();
    } catch (e) {
      throw new Error(`Can't connect to Docker Engine: ${e.message}.\n\nPossible reasons:\n1. Docker is not started\n2. Permission issues ${dockerPostInstallLinuxLink}\n3. Wrong context ${dockerContextLink}`);
    }

    if (typeof dockerVersionInfo === 'string') {
      // Old versions
      const parsedVersion = semver.coerce(dockerVersionInfo);

      if (parsedVersion === null) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't parse version from dockerVersionInfo: ${util.inspect(dockerVersionInfo)}`);
        }

        return;
      }

      if (semver.lt(parsedVersion, DockerCompose.DOCKER_MIN_VERSION)) {
        throw new Error(`Update Docker to version ${DockerCompose.DOCKER_MIN_VERSION} or higher. Please follow instructions ${dockerInstallLink}`);
      }
    } else {
      // Since 1.39
      if (typeof dockerVersionInfo?.Components[0]?.Details?.ApiVersion !== 'string') {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Docker API version must be a string: ${util.inspect(dockerVersionInfo)}`);
        }

        return;
      }

      const parsedVersion = semver.coerce(dockerVersionInfo.Components[0].Details.ApiVersion);

      if (parsedVersion === null) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Can't parse docker API version: ${util.inspect(dockerVersionInfo)}`);
        }

        return;
      }

      const minVersion = '1.25.0';

      if (semver.lt(parsedVersion, minVersion)) {
        throw new Error(`Update Docker Engine to version ${minVersion} or higher. Please follow instructions ${dockerInstallLink}`);
      }
    }
  }

  /**
   * @private
   */
  async throwErrorIfDockerComposeIsNotInstalled() {
    const dockerComposeInstallLink = 'https://docs.docker.com/compose/install/';

    // Check docker compose
    let version;
    try {
      ({ out: version } = await dockerCompose.version());
    } catch (e) {
      throw new Error(`Docker Compose V2 is not available in your system. Please follow instructions ${dockerComposeInstallLink}`);
    }

    if (typeof version !== 'string') {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`docker compose version is not a string: ${util.inspect(version)}`);
      }

      return;
    }

    const parsedVersion = semver.coerce(version);

    if (parsedVersion === null) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Can't parse docker compose version: ${util.inspect(version)}`);
      }

      return;
    }

    if (semver.lt(parsedVersion, DockerCompose.DOCKER_COMPOSE_MIN_VERSION)) {
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

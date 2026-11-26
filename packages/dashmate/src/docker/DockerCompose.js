import { Observable } from 'rxjs';

import dockerCompose from '@dashevo/docker-compose';
import isWsl from 'is-wsl';

import hasbin from 'hasbin';
import util from 'node:util';
import semver from 'semver';

import { PACKAGE_ROOT_DIR } from '../constants.js';
import ContainerIsNotPresentError from './errors/ContainerIsNotPresentError.js';
import DockerComposeError from './errors/DockerComposeError.js';
import ServiceAlreadyRunningError from './errors/ServiceAlreadyRunningError.js';
import ServiceIsNotRunningError from './errors/ServiceIsNotRunningError.js';

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
   * @type {Error}
   */
  #dockerVerifiicationError;

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
   * @param {string[]} [options.profiles] - Filter by profiles, or with
   *  serviceNames the profiles to enable
   * @param {string[]} [options.serviceNames] - Check these services only
   * @return {Promise<boolean>}
   */
  async isNodeRunning(config, options = { profiles: [] }) {
    await this.throwErrorIfNotInstalled();

    if (options.serviceNames) {
      return this.#isAnyServiceRunning(config, options.serviceNames, options.profiles);
    }

    let serviceList = this.#getServiceList(config);

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
   * List the project's running containers and pick the named services here
   *
   * `docker compose ps <names>` fails before Compose 2.15 when a named service
   * has no container, and that read as "not running". Without names, older
   * Compose versions leave out the containers of services whose profile is not
   * enabled, so the profiles to enable come with the names.
   *
   * @param {Config} config
   * @param {string[]} serviceNames
   * @param {string[]} [profiles]
   * @return {Promise<boolean>}
   */
  async #isAnyServiceRunning(config, serviceNames, profiles = []) {
    if (serviceNames.length === 0) {
      return false;
    }

    const containers = await this.getContainersList(config, { profiles });

    return containers.some(({ Service: service, State: state }) => state === 'running'
      && serviceNames.includes(service));
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
      await this.#removeDisabledTor(config, options);
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
      await this.#removeDisabledTor(config, options);
      await dockerCompose.stop(this.#createOptions(config, options));
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * A disabled sidecar is absent from Compose's model, so stop/up cannot remove
   * it. Use project and service labels to find it without touching other nodes
   * or deleting its persistent Tor state. Platform-only operations leave it alone.
   *
   * @param {Config} config
   * @param {Object} [options]
   * @param {string[]} [options.profiles]
   * @return {Promise<void>}
   */
  async #removeDisabledTor(config, { profiles = [] } = {}) {
    if (config.get('core.tor.enabled') || (profiles.length > 0 && !profiles.includes('core'))) {
      return;
    }

    const { COMPOSE_PROJECT_NAME: projectName } = this.#generateEnvs(config);
    const containers = await this.#docker.listContainers({
      all: true,
      filters: {
        label: [
          `com.docker.compose.project=${projectName}`,
          'com.docker.compose.service=core_tor',
        ],
      },
    });

    for (const { Id } of containers) {
      const container = this.#docker.getContainer(Id);
      try {
        try {
          await container.stop({ t: 30 });
        } catch (e) {
          if (e.statusCode !== 304) { // Already stopped containers still need removal.
            throw e;
          }
        }
        await container.remove();
      } catch (e) {
        if (e.statusCode !== 404) { // Another lifecycle operation may have removed it.
          throw e;
        }
      }
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
   * @param {string[]} [options.profiles] - Profiles to enable
   * @return {Promise<object[]>}
   */
  async getContainersList(
    config,
    {
      filterServiceNames = undefined,
      all = false,
      profiles = [],
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
        ...this.#createOptions(config, { profiles }),
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
   * Get the Docker names of the volumes Compose created for the given services
   * and that only they use
   *
   * Docker knows a volume by its name, and several keys of the Compose files
   * can give the same one, so each check is on that name. The project's whole
   * model is read, every profile given enabled: given service names, Compose
   * describes only them and what they depend on, and the services that mount
   * the same volume would go unseen.
   *
   * A volume is left out when any key that names it is external, since the
   * operator manages it, and when a service other than the given ones mounts
   * it under any key, Core included, since removing it would take that
   * service's data.
   *
   * A mounted volume is not necessarily the project's own: an override can
   * mount one that another project created under the same explicit name. Only
   * a volume Docker labels as created by this project for one of its keys is
   * returned.
   *
   * @param {Config} config
   * @param {Object} options
   * @param {string[]} options.serviceNames
   * @param {string[]} [options.profiles] - Profiles to enable, so that every
   *  service of the Compose files is part of the model
   * @return {Promise<string[]>}
   */
  async getVolumeNames(config, { serviceNames, profiles = [] }) {
    if (serviceNames.length === 0) {
      return [];
    }

    await this.throwErrorIfNotInstalled();

    let project;
    try {
      const { out } = await dockerCompose.execCompose('config', [], {
        ...this.#createOptions(config, { profiles }),
        commandOptions: ['--format', 'json'],
      });

      project = JSON.parse(out);
    } catch (e) {
      throw new DockerComposeError(e);
    }

    const { services = {}, volumes = {} } = project;
    const projectName = project.name ?? this.#generateEnvs(config).COMPOSE_PROJECT_NAME;

    // Without the service its volumes are unknown, and skipping it would
    // leave them in place unnoticed.
    serviceNames.forEach((serviceName) => {
      if (!services[serviceName]) {
        throw new Error(`Docker Compose did not describe service '${serviceName}' in 'docker compose config'`);
      }
    });

    const getDockerName = (key) => volumes[key]?.name ?? `${projectName}_${key}`;

    const getMountedKeys = (service) => (service.volumes ?? [])
      .filter((mount) => mount.type === 'volume' && mount.source)
      .map((mount) => mount.source);

    // Docker name to every key of the model that names it
    const keysByName = new Map();
    new Set([...Object.keys(volumes), ...Object.values(services).flatMap(getMountedKeys)])
      .forEach((key) => {
        const name = getDockerName(key);

        keysByName.set(name, (keysByName.get(name) ?? new Set()).add(key));
      });

    const namesMountedElsewhere = new Set(Object.entries(services)
      .filter(([serviceName]) => !serviceNames.includes(serviceName))
      .flatMap(([, service]) => getMountedKeys(service))
      .map(getDockerName));

    const candidates = new Set(serviceNames
      .flatMap((serviceName) => getMountedKeys(services[serviceName]))
      .map(getDockerName));

    const ownedNames = await Promise.all(
      Array.from(candidates)
        .filter((name) => !namesMountedElsewhere.has(name))
        .filter((name) => !Array.from(keysByName.get(name))
          .some((key) => volumes[key]?.external))
        .map(async (name) => {
          let labels;
          try {
            ({ Labels: labels } = await this.#docker.getVolume(name).inspect());
          } catch (e) {
            // Never created, so nothing to remove
            if (e.statusCode === 404) {
              return null;
            }

            throw e;
          }

          const isOwned = labels?.['com.docker.compose.project'] === projectName
            && keysByName.get(name).has(labels?.['com.docker.compose.volume']);

          return isOwned ? name : null;
        }),
    );

    return ownedNames.filter((name) => name !== null);
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
  async rm(config, {
    serviceNames = [],
    profiles = [],
    removeVolumes = false,
    force = false,
  } = {}) {
    await this.throwErrorIfNotInstalled();

    const commandOptions = ['--stop'];

    if (removeVolumes) {
      commandOptions.push('-v');
    }

    if (force) {
      commandOptions.push('--force');
    }

    try {
      await dockerCompose.rm({
        ...this.#createOptions(config, { profiles }),
        commandOptions,
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
   * Logs
   *
   * @param {Config} config
   * @param {string[]} services
   * @param {Object} options
   * @param {number} options.tail
   * @return {Promise<{exitCode: number | null, out: string, err: string}>}
   */
  async logs(config, services = [], options = {}) {
    await this.throwErrorIfNotInstalled();

    const args = [...services];
    if (options.tail) {
      args.unshift('--tail', options.tail.toString());
    }

    const commandOptions = this.#createOptions(config);

    try {
      return await dockerCompose.logs(args, commandOptions);
    } catch (e) {
      throw new DockerComposeError(e);
    }
  }

  /**
   * @return {Promise<void>}
   */
  async throwErrorIfNotInstalled() {
    if (this.#isDockerSetupVerified) {
      if (this.#dockerVerifiicationError) {
        throw this.#dockerVerifiicationError;
      } else {
        return;
      }
    }

    try {
      await this.throwErrorIfDockerIsNotInstalled();

      await this.throwErrorIfDockerComposeIsNotInstalled();
    } catch (e) {
      this.#dockerVerifiicationError = e;

      throw e;
    } finally {
      this.#isDockerSetupVerified = true;
    }
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

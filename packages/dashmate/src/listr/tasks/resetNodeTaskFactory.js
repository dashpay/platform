import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import wait from '../../util/wait.js';
import resolveConfigDirectory from '../../config/resolve-config-directory.js';
import ensureTenderdashNodeKey from '../../tenderdash/ensureTenderdashNodeKey.js';

// More paths than this are counted rather than listed.
const MAX_REPORTED_PATHS = 10;

// A volume still in use after this many removals is held by a container the
// reset does not remove.
const MAX_VOLUME_REMOVAL_ATTEMPTS = 10;

// The helper keeps certificate renewal state in the Platform directory, so a
// Platform reset removes it with the Platform services; start brings it back.
const HELPER_SERVICE_NAME = 'dashmate_helper';

/**
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {generateToAddressTask} generateToAddressTask
 * @param {DefaultConfigs} defaultConfigs
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @param {getServiceList} getServiceList
 * @return {resetNodeTask}
 */
export default function resetNodeTaskFactory(
  dockerCompose,
  docker,
  startNodeTask,
  generateToAddressTask,
  defaultConfigs,
  configFile,
  homeDir,
  getServiceList,
) {
  /**
   * Select every Platform service of the config, whether the config enables it
   * or not
   *
   * A Platform-only reset names them rather than selecting their profiles.
   * A profile selection leaves Core out of the Compose model, and a Platform
   * service that depends on Core (the quorum list) then makes the project
   * invalid: Compose fails, or for rm and stop acts on the whole project, Core
   * included. Every profile is still enabled next to the names, because some
   * Compose versions (2.10 to 2.12) skip a named service whose profile is not
   * enabled. With every service in the model, Compose acts on the named ones
   * only.
   *
   * @param {Config} config
   * @return {{ serviceNames: string[], profiles: string[] }}
   */
  function selectPlatformServices(config) {
    const services = getServiceList(config, { allProfiles: true });

    const serviceNames = services
      .filter(({ profiles }) => profiles.some((profile) => profile.startsWith('platform')))
      .map(({ name }) => name);

    // Compose acts on every service of the project when given no names.
    if (serviceNames.length === 0) {
      throw new Error('No Platform services found in the Compose files');
    }

    const profiles = Array.from(new Set(services.flatMap((service) => service.profiles)));

    return { serviceNames, profiles };
  }

  /**
   * @param {Error} e
   * @return {boolean}
   */
  function isPermissionError(e) {
    return e?.code === 'EACCES' || e?.code === 'EPERM';
  }

  /**
   * Remove a path and everything under it, as far as permissions allow
   *
   * Removal goes on past what it may not remove, so one run removes all it can
   * and returns every path left behind. The entries of a directory this user
   * cannot write cannot be removed, so such a directory is returned once rather
   * than entry by entry, unless it is empty: removing an empty directory needs
   * only its parent to be writable. Symbolic links are removed, never followed.
   *
   * @param {string} targetPath
   * @return {string[]} paths left in place for lack of permission
   */
  function removePathAsFarAsPermitted(targetPath) {
    const deniedPaths = [];

    /**
     * @param {string} currentPath
     * @return {boolean} whether the path is gone
     */
    function remove(currentPath) {
      let stats;
      try {
        stats = fs.lstatSync(currentPath);
      } catch (e) {
        if (e.code === 'ENOENT') {
          return true;
        }

        throw e;
      }

      const isDirectory = stats.isDirectory();
      let canRemoveEntries = true;

      if (isDirectory) {
        try {
          // eslint-disable-next-line no-bitwise
          fs.accessSync(currentPath, fs.constants.R_OK | fs.constants.W_OK | fs.constants.X_OK);
        } catch (e) {
          if (!isPermissionError(e)) {
            throw e;
          }

          canRemoveEntries = false;
        }

        if (canRemoveEntries) {
          const isEmptied = fs.readdirSync(currentPath)
            .map((entry) => remove(path.join(currentPath, entry)))
            .every((isRemoved) => isRemoved);

          if (!isEmptied) {
            return false;
          }
        }
      }

      try {
        if (isDirectory) {
          fs.rmdirSync(currentPath);
        } else {
          fs.unlinkSync(currentPath);
        }
      } catch (e) {
        if (e.code === 'ENOENT') {
          return true;
        }

        // A directory whose entries it could not remove still has them.
        const isLockedDirectory = !canRemoveEntries
          && (e.code === 'ENOTEMPTY' || e.code === 'EEXIST');

        if (isPermissionError(e) || isLockedDirectory) {
          deniedPaths.push(currentPath);

          return false;
        }

        throw e;
      }

      return true;
    }

    remove(targetPath);

    return deniedPaths;
  }

  /**
   * Remove a path and everything under it, or fail naming what is left
   *
   * The config is saved only when the command succeeds, so after this error
   * config.json is as it was and the reset can be run again.
   *
   * @param {string} targetPath
   */
  function removePath(targetPath) {
    const deniedPaths = removePathAsFarAsPermitted(targetPath);

    if (deniedPaths.length === 0) {
      return;
    }

    const shownPaths = deniedPaths.slice(0, MAX_REPORTED_PATHS)
      .map((deniedPath) => `  '${deniedPath}'`);

    if (deniedPaths.length > MAX_REPORTED_PATHS) {
      shownPaths.push(`  and ${deniedPaths.length - MAX_REPORTED_PATHS} more`);
    }

    // Starting the node first would let Docker create the removed mount
    // sources again, as root.
    throw new Error([
      `Could not remove ${deniedPaths.length} path(s) under '${targetPath}' for lack of permission:`,
      ...shownPaths,
      'They are probably owned by root, created by a container. Remove them with sufficient',
      'rights, or fix their ownership or permissions, for example:',
      `  sudo rm -rf '${targetPath}'`,
      'then run the reset again. Do not start the node before the reset has completed.',
    ].join('\n'));
  }

  /**
   * @typedef {resetNodeTask}
   * @param {Config} config
   */
  function resetNodeTask(config) {
    return new Listr([
      {
        task: (ctx) => {
          // A hard reset may still clear the Platform data a node ran with before
          // Platform was disabled, as a mainnet hard reset leaves it.
          if (!config.get('platform.enable') && ctx.isPlatformOnlyReset && !ctx.isHardReset) {
            throw new Error('Cannot reset platform only if platform services are not enabled in config');
          }

          // A hard Platform reset restores the Platform options from the defaults
          // of the config's preset or group. Without them there is nothing to
          // restore, so refuse before any service is touched rather than fall
          // back to removing the whole config, Core included.
          const defaultConfigName = config.get('group') || config.getName();

          if (ctx.isPlatformOnlyReset && ctx.isHardReset && !ctx.removeConfig
            && !defaultConfigs.has(defaultConfigName)) {
            const name = config.getName();

            throw new Error([
              `Cannot reset Platform only for config '${name}': dashmate has no Platform defaults`
                + ` for '${defaultConfigName}' to restore.`,
              'To remove Platform services and data and keep the config, run it without --hard:',
              `  dashmate reset --config ${name} --platform`,
              'and set Platform options with \'dashmate config set\'.',
              'To remove the whole config, Core included:',
              `  dashmate reset --config ${name} --hard`,
            ].join('\n'));
          }
        },
      },
      {
        title: 'Check services are not running',
        skip: (ctx) => ctx.isForce,
        task: async (ctx) => {
          const options = ctx.isPlatformOnlyReset
            ? selectPlatformServices(config)
            : { profiles: [] };

          if (await dockerCompose.isNodeRunning(config, options)) {
            throw new Error('Running services detected. Please ensure all services are stopped for this config before starting');
          }
        },
      },
      {
        title: 'Remove all services and associated data',
        enabled: (ctx) => !ctx.isPlatformOnlyReset,
        task: async (ctx, task) => {
          if (ctx.keepData) {
            // eslint-disable-next-line no-param-reassign
            task.title = 'Remove all services and keep associated data';
          }

          const options = {
            removeVolumes: !ctx.keepData,
          };

          return dockerCompose.down(config, options);
        },
      },
      {
        title: 'Remove platform services and associated data',
        enabled: (ctx) => ctx.isPlatformOnlyReset,
        task: async (ctx, task) => {
          const { serviceNames, profiles } = selectPlatformServices(config);

          const removeOptions = {
            serviceNames: [...serviceNames, HELPER_SERVICE_NAME],
            profiles,
            // Anonymous volumes go with their containers unless data is kept.
            removeVolumes: !ctx.keepData,
          };

          if (ctx.keepData) {
            // eslint-disable-next-line no-param-reassign
            task.title = 'Remove platform services and keep associated data';
          }

          // Looked up first: Compose fails to describe the project when service
          // files are missing, and then nothing has been removed yet.
          const volumeNames = ctx.keepData
            ? []
            : await dockerCompose.getVolumeNames(config, { serviceNames, profiles });

          await dockerCompose.rm(config, removeOptions);

          // Remove volumes
          if (!ctx.keepData) {
            await Promise.all(
              volumeNames
                .map(async (volumeName) => {
                  const volume = await docker.getVolume(volumeName);

                  let attempts = 0;
                  let isRetry;
                  do {
                    isRetry = false;
                    attempts += 1;

                    try {
                      await volume.remove({ force: true });
                    } catch (e) {
                      // volume is in use
                      if (e.statusCode === 409) {
                        if (attempts >= MAX_VOLUME_REMOVAL_ATTEMPTS) {
                          throw new Error(`Volume '${volumeName}' is still in use by a container this`
                            + ' reset does not remove. Find it with'
                            + ` 'docker ps -a --filter volume=${volumeName}', remove it, and run the`
                            + ' reset again.');
                        }

                        await wait(1000);

                        // Remove containers
                        await dockerCompose.rm(config, removeOptions);

                        isRetry = true;

                        continue;
                      }

                      // volume does not exist
                      if (e.statusCode === 404) {
                        break;
                      }

                      throw e;
                    }
                  } while (isRetry);
                }),
            );
          }
        },
      },
      {
        title: `Remove config ${config.getName()}`,
        enabled: (ctx) => ctx.removeConfig,
        task: () => {
          const serviceConfigsPath = resolveConfigDirectory(homeDir, config.getName());

          configFile.removeConfig(config.getName());

          removePath(serviceConfigsPath);
        },
      },
      {
        title: `Reset config ${config.getName()}`,
        enabled: (ctx) => !ctx.removeConfig && ctx.isHardReset,
        task: (ctx) => {
          const groupName = config.get('group');
          const defaultConfigName = groupName || config.getName();

          // The group name only selects the defaults. Each member's service
          // files live in a directory named after the config itself.
          const configDirectory = resolveConfigDirectory(homeDir, config.getName());

          if (defaultConfigs.has(defaultConfigName)) {
            // Reset config if the corresponding default config exists
            if (ctx.isPlatformOnlyReset) {
              const defaultPlatformConfig = defaultConfigs.get(defaultConfigName).getStored('platform');
              config.set('platform', defaultPlatformConfig);
            } else {
              const defaultConfigOptions = defaultConfigs.get(defaultConfigName).getStoredOptions();

              config.setOptions(defaultConfigOptions);
            }

            config.set('group', groupName);

            // Resetting platform options clears a fullnode's node identity.
            // Give it a new one now, so it is saved with the reset config
            // rather than generated later at render time.
            ensureTenderdashNodeKey(config);

            // Remove service configs
            let serviceConfigsPath = configDirectory;

            if (ctx.isPlatformOnlyReset) {
              serviceConfigsPath = path.join(serviceConfigsPath, 'platform');
            }

            removePath(serviceConfigsPath);
          } else {
            // Delete config if no base config
            configFile.removeConfig(config.getName());

            // Remove service configs
            removePath(configDirectory);
          }
        },
      },
    ]);
  }

  return resetNodeTask;
}

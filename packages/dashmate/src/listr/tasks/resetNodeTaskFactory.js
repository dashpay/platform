import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import wait from '../../util/wait.js';
import resolveConfigDirectory from '../../config/resolve-config-directory.js';
import ensureTenderdashNodeKey from '../../tenderdash/ensureTenderdashNodeKey.js';

// More paths than this are counted rather than listed.
const MAX_REPORTED_PATHS = 10;

/**
 * @param {DockerCompose} dockerCompose
 * @param {Docker} docker
 * @param {startNodeTask} startNodeTask
 * @param {generateToAddressTask} generateToAddressTask
 * @param {DefaultConfigs} defaultConfigs
 * @param {ConfigFile} configFile
 * @param {HomeDir} homeDir
 * @param {generateEnvs} generateEnvs
 * @param {getConfigProfiles} getConfigProfiles
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
  generateEnvs,
  getConfigProfiles,
) {
  function selectPlatformProfiles(config, options) {
    return getConfigProfiles(config, options)
      .filter((profile) => profile.startsWith('platform'));
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
          if (!config.get('platform.enable') && ctx.isPlatformOnlyReset) {
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
          const profiles = ctx.isPlatformOnlyReset
            ? selectPlatformProfiles(config, { includeAll: true })
            : [];

          if (await dockerCompose.isNodeRunning(
            config,
            { profiles },
          )) {
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
          const profiles = selectPlatformProfiles(config, { includeAll: true });

          if (ctx.keepData) {
            // eslint-disable-next-line no-param-reassign
            task.title = 'Remove platform services and keep associated data';
          }

          await dockerCompose.rm(config, { profiles });

          // Remove volumes
          if (!ctx.keepData) {
            const { COMPOSE_PROJECT_NAME: composeProjectName } = generateEnvs(config);

            const projectVolumeNames = await dockerCompose.getVolumeNames(
              config,
              { profiles },
            );

            await Promise.all(
              projectVolumeNames
                .map((volumeName) => `${composeProjectName}_${volumeName}`)
                .map(async (volumeName) => {
                  const volume = await docker.getVolume(volumeName);

                  let isRetry;
                  do {
                    isRetry = false;

                    try {
                      await volume.remove({ force: true });
                    } catch (e) {
                      // volume is in use
                      if (e.statusCode === 409) {
                        await wait(1000);

                        // Remove containers
                        await dockerCompose.rm(config, { profiles });

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

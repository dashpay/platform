import lodash from 'lodash';
import isServiceBuildRequired from './isServiceBuildRequired.js';

/**
 * The images a build produces from local sources
 *
 * The build compose files tag each one with a fixed local tag (`drive:local`,
 * `rs-dapi:local`, ...) whatever image a node is configured with, so on one host
 * each is a single image that every node of the group runs.
 */
const LOCAL_IMAGES = [
  {
    name: 'base',
    buildPath: 'docker.baseImage.build',
    isBuilt: (config) => config.get('docker.baseImage.build.enabled'),
  },
  {
    name: 'dashmate helper',
    buildPath: 'dashmate.helper.docker.build',
    isBuilt: (config) => config.get('dashmate.helper.docker.build.enabled'),
  },
  {
    name: 'drive',
    buildPath: 'platform.drive.abci.docker.build',
    isBuilt: (config) => config.get('platform.enable')
      && config.get('platform.drive.abci.docker.build.enabled'),
  },
  {
    name: 'rs-dapi',
    buildPath: 'platform.dapi.rsDapi.docker.build',
    isBuilt: (config) => config.get('platform.enable')
      && config.get('platform.dapi.rsDapi.docker.build.enabled'),
  },
];

/**
 * Pick the configs of a group to build local images for
 *
 * Each node's own config decides which images it builds from local sources, so
 * no single node can stand for the group. A node is built for only when it builds
 * an image no node before it does: building it again would produce the same image.
 *
 * @param {Config[]} configGroup
 * @return {Config[]}
 * @throws {Error} when two nodes build the same image with different settings,
 *                 since only one of the two images could exist
 */
export default function selectGroupBuildConfigs(configGroup) {
  const buildConfigs = configGroup.filter(isServiceBuildRequired);

  const firstBuilds = new Map();

  buildConfigs.forEach((config) => {
    LOCAL_IMAGES.filter((image) => image.isBuilt(config)).forEach((image) => {
      const settings = config.get(image.buildPath) ?? null;
      const firstBuild = firstBuilds.get(image.name);

      if (!firstBuild) {
        firstBuilds.set(image.name, { config, settings });
      } else if (!lodash.isEqual(firstBuild.settings, settings)) {
        throw new Error(`${firstBuild.config.getName()} and ${config.getName()} build the ${image.name} image `
          + 'differently, but the nodes of a group share one local image of it. '
          + `Make its build settings (${image.buildPath}) the same for both nodes.`);
      }
    });
  });

  const builtImages = new Set();

  return buildConfigs.filter((config) => {
    const images = LOCAL_IMAGES.filter((image) => image.isBuilt(config)).map(({ name }) => name);

    if (images.every((name) => builtImages.has(name))) {
      return false;
    }

    images.forEach((name) => builtImages.add(name));

    return true;
  });
}

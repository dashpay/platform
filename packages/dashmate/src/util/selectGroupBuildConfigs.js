import isServiceBuildRequired from './isServiceBuildRequired.js';

/**
 * Describe the images a config builds from local sources
 *
 * The images are tagged and built from what the config says about them, so two
 * configs with the same description build the same images.
 *
 * @param {Config} config
 * @return {string}
 */
function describeBuild(config) {
  const isPlatformEnabled = config.get('platform.enable');

  const build = (isEnabled, dockerPath) => (isEnabled ? { docker: config.get(dockerPath) } : null);

  return JSON.stringify([
    config.get('docker.baseImage'),
    build(config.get('dashmate.helper.docker.build.enabled'), 'dashmate.helper.docker'),
    build(
      isPlatformEnabled && config.get('platform.drive.abci.docker.build.enabled'),
      'platform.drive.abci.docker',
    ),
    build(
      isPlatformEnabled && config.get('platform.dapi.rsDapi.docker.build.enabled'),
      'platform.dapi.rsDapi.docker',
    ),
  ]);
}

/**
 * Pick the configs of a group to build local images for
 *
 * Each node's own config decides which of its services are built from local
 * sources, so no single node can stand for the group. Nodes that describe the
 * same build would only build the same images again, so each build is kept once.
 *
 * @param {Config[]} configGroup
 * @return {Config[]}
 */
export default function selectGroupBuildConfigs(configGroup) {
  const describedBuilds = new Set();

  return configGroup.filter((config) => {
    if (!isServiceBuildRequired(config)) {
      return false;
    }

    const build = describeBuild(config);

    if (describedBuilds.has(build)) {
      return false;
    }

    describedBuilds.add(build);

    return true;
  });
}

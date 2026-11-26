import fs from 'fs';
import path from 'path';
import selectPlatformServices from '../docker/selectPlatformServices.js';

// Container states in which a service is not doing anything.
const STOPPED_STATES = ['exited', 'created'];

const CERTIFICATE_FILE_NAMES = ['bundle.crt', 'private.key'];

/**
 * The gateway certificate files of a config that exist
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {string[]}
 */
export function findCertificateFiles(homeDir, configName) {
  const sslDir = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl');

  return CERTIFICATE_FILE_NAMES
    .map((fileName) => path.join(sslDir, fileName))
    .filter((filePath) => fs.existsSync(filePath));
}

/**
 * @param {HomeDir} homeDir
 * @param {DockerCompose} dockerCompose
 * @param {getServiceList} getServiceList
 * @return {getPlatformArtefacts}
 */
export default function getPlatformArtefactsFactory(
  homeDir,
  dockerCompose,
  getServiceList,
) {
  /**
   * The Platform services whose containers are not stopped
   *
   * A restarting or paused container comes back by itself, so only exited
   * and never started ones count as stopped.
   *
   * @param {Config} config
   * @return {Promise<string[]>}
   */
  async function getActivePlatformServices(config) {
    const { serviceNames, profiles } = selectPlatformServices(getServiceList, config);

    const containers = await dockerCompose.getContainersList(config, { all: true, profiles });

    return Array.from(new Set(
      containers
        .filter(({ Service: service, State: state }) => serviceNames.includes(service)
          && !STOPPED_STATES.includes(state))
        .map(({ Service: service }) => service),
    ));
  }

  /**
   * What a Platform that ran leaves outside config.json, and which of its
   * services are not stopped
   *
   * The services are the ones a Platform-only reset removes, every Platform
   * profile included, so a service the config does not enable today is still
   * seen. Docker that cannot be reached is an error, never "nothing there".
   *
   * @typedef {function} getPlatformArtefacts
   * @param {Config} config
   * @return {Promise<{
   *   certificateFiles: string[],
   *   dataVolumes: string[],
   *   activeServices: string[],
   * }>}
   */
  async function getPlatformArtefacts(config) {
    await dockerCompose.throwErrorIfNotInstalled();

    const { serviceNames, profiles } = selectPlatformServices(getServiceList, config);

    const certificateFiles = findCertificateFiles(homeDir, config.getName());

    // The volumes that exist and that Compose created for this project: what
    // a reset that kept data leaves, and what a Platform-only reset removes.
    // An external volume, or another project's, is neither.
    const dataVolumes = await dockerCompose.getVolumeNames(config, { serviceNames, profiles });

    const activeServices = await getActivePlatformServices(config);

    return { certificateFiles, dataVolumes, activeServices };
  }

  getPlatformArtefacts.getActiveServices = getActivePlatformServices;

  return getPlatformArtefacts;
}

import fs from 'fs';
import path from 'path';
import selectPlatformServices from '../docker/selectPlatformServices.js';
import findCertificatePairProblem from '../ssl/findCertificatePairProblem.js';

// Container states in which a service is not doing anything.
const STOPPED_STATES = ['exited', 'created'];

const CERTIFICATE_FILE_NAMES = ['bundle.crt', 'private.key'];

// A Platform-only reset removes the helper with the Platform services, so a
// volume they share with it is one the reset removes.
const HELPER_SERVICE_NAME = 'dashmate_helper';

/**
 * @param {string} filePath
 * @return {boolean} false only when there is no such entry
 */
function isPresent(filePath) {
  try {
    fs.lstatSync(filePath);

    return true;
  } catch (e) {
    return e.code !== 'ENOENT' && e.code !== 'ENOTDIR';
  }
}

/**
 * The gateway certificate files of a config that exist
 *
 * An entry counts whether or not this process can read what it points to:
 * Docker running as root mounts files the operator's user cannot read, and a
 * file this process cannot look at is no proof that there is none.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {string[]}
 */
export function findCertificateFiles(homeDir, configName) {
  const sslDir = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl');

  return CERTIFICATE_FILE_NAMES
    .map((fileName) => path.join(sslDir, fileName))
    .filter(isPresent);
}

/**
 * @param {string} filePath
 * @return {{ kind: string, content?: Buffer }} kind is 'file' with its
 *  content, 'missing', 'directory', 'other', or 'unknown' when this process
 *  cannot look
 */
function inspectFile(filePath) {
  let stats;
  try {
    stats = fs.statSync(filePath);
  } catch (e) {
    // A link that points nowhere is missing too.
    return { kind: e.code === 'ENOENT' || e.code === 'ENOTDIR' ? 'missing' : 'unknown' };
  }

  if (stats.isDirectory()) {
    return { kind: 'directory' };
  }

  if (!stats.isFile()) {
    return { kind: 'other' };
  }

  try {
    return { kind: 'file', content: fs.readFileSync(filePath) };
  } catch {
    return { kind: 'unknown' };
  }
}

/**
 * Whether the gateway can serve TLS from the certificate files of a config,
 * as far as this process can tell
 *
 * The gateway bind-mounts both files. A missing one, a link that points
 * nowhere and a directory, which Docker creates for a missing mount source,
 * serve nothing, and neither does a pair that does not load or match or whose
 * certificate expired. A file this process cannot read counts as usable:
 * Docker running as root mounts files the operator's user cannot read.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {{ isUsable: boolean, directories: string[] }} directories stand
 *  where a file goes
 */
export function inspectCertificateFiles(homeDir, configName) {
  const sslDir = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl');

  const paths = CERTIFICATE_FILE_NAMES.map((fileName) => path.join(sslDir, fileName));
  const [bundle, key] = paths.map(inspectFile);

  const directories = paths.filter((filePath, index) => [bundle, key][index].kind === 'directory');

  let isUsable = [bundle, key].every(({ kind }) => kind === 'file' || kind === 'unknown');

  if (isUsable && bundle.kind === 'file' && key.kind === 'file') {
    isUsable = findCertificatePairProblem(
      bundle.content.toString('utf8'),
      key.content.toString('utf8'),
    ) === null;
  }

  return { isUsable, directories };
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

    const { serviceNames } = selectPlatformServices(getServiceList, config);

    const certificateFiles = findCertificateFiles(homeDir, config.getName());

    // The volumes a Platform-only reset removes, found the way it finds them:
    // what a reset that kept data leaves, and what `reset --platform` then
    // clears. An external volume, one that Core uses too, or another
    // project's is none of these.
    const candidates = await dockerCompose.getVolumeCandidates(config, {
      serviceNames,
      removedServiceNames: [...serviceNames, HELPER_SERVICE_NAME],
    });

    const ownership = await Promise.all(
      candidates.map((candidate) => dockerCompose.isVolumeOwned(candidate)),
    );

    const dataVolumes = candidates
      .filter((candidate, index) => ownership[index])
      .map(({ name }) => name);

    const activeServices = await getActivePlatformServices(config);

    return { certificateFiles, dataVolumes, activeServices };
  }

  getPlatformArtefacts.getActiveServices = getActivePlatformServices;

  return getPlatformArtefacts;
}

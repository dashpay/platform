import chalk from 'chalk';
import semver from 'semver';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';
import renderConfigFlag from '../../util/renderConfigFlag.js';
import { ACTIVATION_REASON, ACTIVATION_STATE } from '../../update/probeActivationState.js';

/**
 * @param {*} value
 * @return {boolean}
 */
function isNonEmptyString(value) {
  return typeof value === 'string' && value.length > 0;
}

/**
 * @param {Object} sampledConfig
 * @return {string}
 */
function renderRestartCommand(sampledConfig) {
  // Without --safe the stop refuses inside a DKG window, so a masternode is restarted in the
  // next window that is safe for it instead.
  const safe = sampledConfig.masternode === true ? ' --safe' : '';

  return chalk`{bold.cyanBright dashmate restart ${renderConfigFlag(sampledConfig.name)}${safe}}`;
}

/**
 * The version a helper image is tagged with, or null when the tag is not a version.
 *
 * @param {*} image
 * @return {string|null}
 */
function helperImageVersion(image) {
  if (!isNonEmptyString(image)) {
    return null;
  }

  const tag = image.slice(image.lastIndexOf(':') + 1);

  return tag.includes('/') ? null : semver.valid(tag);
}

/**
 * Whether the helper of a pending node runs a newer dashmate than the one that collected the
 * samples - that is, the node is ahead of this dashmate rather than behind it.
 *
 * @param {Object} sampledConfig
 * @return {boolean}
 */
function isHelperNewer(sampledConfig) {
  const running = helperImageVersion(sampledConfig.helper?.runningImage);
  const expected = helperImageVersion(sampledConfig.helper?.expectedImage);

  return running !== null && expected !== null && semver.gt(running, expected);
}

/**
 * @param {Object} sampledConfig
 * @param {string} version
 * @return {string}
 */
function restartSolution(sampledConfig, version) {
  // The restart makes the gateway load the certificate files on disk, which can be worse than
  // the certificate it serves now; that is judged per node by its own doctor run.
  const certificateCheck = sampledConfig.platform === true
    ? chalk`The restart loads the gateway certificate files on disk. If {bold.cyanBright dashmate doctor ${renderConfigFlag(sampledConfig.name)}}
reports a problem with them, fix it first.
`
    : '';

  return chalk`Dashmate was upgraded, but this node still runs services started by the previous version.
${certificateCheck}Restart it on ${version}:
  ${renderRestartCommand(sampledConfig)}`;
}

/**
 * @param {Object} sampledConfig
 * @param {string} version
 * @param {string} message
 * @return {Problem}
 */
function unknownProblem(sampledConfig, version, message) {
  // A tag every build from source shares cannot say which dashmate started the node, so no
  // restart will ever settle the question.
  const solution = sampledConfig.reason === ACTIVATION_REASON.HELPER_BUILT_LOCALLY
    ? chalk`This node builds its helper from source, so its version cannot be checked.
Rebuild and restart it after updating the sources.`
    : chalk`If dashmate was upgraded since this node was last started, restart it on ${version}:
  ${renderRestartCommand(sampledConfig)}`;

  return new Problem(
    chalk`Could not determine whether node {bold ${sampledConfig.name}} runs dashmate ${version}:
${message}`,
    solution,
    SEVERITY.LOW,
  );
}

/**
 * @param {*} activation
 * @return {boolean}
 */
function isWellFormed(activation) {
  return activation !== null
    && typeof activation === 'object'
    && Array.isArray(activation.configs)
    && activation.configs.every((sampledConfig) => (
      sampledConfig !== null
      && typeof sampledConfig === 'object'
      && isNonEmptyString(sampledConfig.name)
    ));
}

/**
 * @return {analyseActivation}
 */
export default function analyseActivationFactory() {
  /**
   * Report every node on the host that is not yet running the dashmate that collected the
   * samples.
   *
   * The verdict for each config was taken where the node runs, by `probeActivationState`; this
   * only turns it into advice. Anything that is not positively activated is reported - a state
   * that cannot be read, or that this dashmate does not recognise, as unknown and never as fine.
   *
   * @typedef {Function} analyseActivation
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analyseActivation(samples) {
    const activation = samples.getActivation();

    // A report collected by a dashmate that predates this check carries no verdict either way.
    if (activation === undefined) {
      return [];
    }

    // The version that collected the samples, because a report is analysed wherever it is sent.
    const version = isNonEmptyString(activation?.dashmateVersion)
      ? activation.dashmateVersion
      : 'the installed version';

    if (isNonEmptyString(activation?.error)) {
      // Docker being unavailable is already reported, with its own remedy
      if (samples.getStringifiedDockerError()) {
        return [];
      }

      return [new Problem(
        `Could not check whether the nodes on this host run dashmate ${version}:
${activation.error}`,
        'Fix the problem above and run the doctor again.',
        SEVERITY.LOW,
      )];
    }

    if (!isWellFormed(activation)) {
      return [new Problem(
        `Could not check whether the nodes on this host run dashmate ${version}:
the collected activation sample is malformed`,
        'Collect the samples again with this version of dashmate.',
        SEVERITY.LOW,
      )];
    }

    // The config file was migrated by a newer dashmate than the one that collected the samples.
    // Every node on the host may then run services newer than this dashmate pins, so restarting
    // any of them on it would take the node back to older services.
    if (activation.migration?.reason === ACTIVATION_REASON.CONFIG_FORMAT_AHEAD) {
      return [new Problem(
        `The config file was written by a newer dashmate than ${version}:
${isNonEmptyString(activation.migration.message) ? activation.migration.message : 'no explanation was recorded'}`,
        `Do not restart the nodes on this host with dashmate ${version}.
Run the newer dashmate that wrote the config file, or upgrade this one to it.`,
        SEVERITY.MEDIUM,
      )];
    }

    const problems = [];

    for (const sampledConfig of activation.configs) {
      const message = isNonEmptyString(sampledConfig.message)
        ? sampledConfig.message
        : 'no explanation was recorded';

      // Only when the missing helper is what decided the verdict: definite evidence from the
      // config file outranks it.
      if (sampledConfig.reason === ACTIVATION_REASON.HELPER_NOT_RUNNING) {
        // A stopped node runs nothing an earlier dashmate started, and starts on this one.
        if (sampledConfig.nodeRunning === false) {
          continue;
        }

        problems.push(unknownProblem(
          sampledConfig,
          version,
          sampledConfig.nodeRunning === true
            ? 'its helper container is not running while other services of this node are'
            : 'its helper container is not running, and whether the rest of the node is could not be read',
        ));

        continue;
      }

      // Activated needs the helper's own confirmation too, which every collected sample carries
      if (sampledConfig.state === ACTIVATION_STATE.ACTIVATED
        && sampledConfig.helper?.state === ACTIVATION_STATE.ACTIVATED) {
        continue;
      }

      if (sampledConfig.state === ACTIVATION_STATE.PENDING && isHelperNewer(sampledConfig)) {
        problems.push(new Problem(
          chalk`Node {bold ${sampledConfig.name}} runs a newer dashmate than ${version}:
${message}`,
          `Do not restart it with dashmate ${version}, which would take it back to older services.
Run the newer dashmate that started it, or upgrade this one to it.`,
          SEVERITY.MEDIUM,
        ));

        continue;
      }

      if (sampledConfig.state === ACTIVATION_STATE.PENDING) {
        problems.push(new Problem(
          chalk`Node {bold ${sampledConfig.name}} is not running dashmate ${version} yet:
${message}`,
          restartSolution(sampledConfig, version),
          SEVERITY.MEDIUM,
        ));

        continue;
      }

      problems.push(unknownProblem(sampledConfig, version, message));
    }

    return problems;
  }

  return analyseActivation;
}

import fs from 'fs';
import renderConfigFlag from '../util/renderConfigFlag.js';
import getPlatformSetupState, {
  PLATFORM_SETUP_STATE,
  describePlatformSetupState,
} from './getPlatformSetupState.js';
import { findCertificateFiles, inspectCertificateFiles } from './getPlatformArtefactsFactory.js';
import parseTenderdashNodeKey from '../tenderdash/parseTenderdashNodeKey.js';
import findLocalGroupConfigsNotRebuilt, { LOCAL_GROUP_MEMBER_NAME } from './findLocalGroupConfigsNotRebuilt.js';
import { PRESET_LOCAL, PRESET_MAINNET, PRESET_TESTNET } from '../constants.js';
import quoteShellArgument, { isShellQuotable } from '../util/quoteShellArgument.js';

const PLATFORM_PRESETS = [PRESET_MAINNET, PRESET_TESTNET];

/**
 * @param {HomeDir} homeDir
 * @param {DefaultConfigs} defaultConfigs
 * @param {ConfigFile} configFile
 * @return {assertPlatformCanStart}
 */
export default function assertPlatformCanStartFactory(homeDir, defaultConfigs, configFile) {
  /**
   * The node ID Tenderdash will start with: the one its rendered key derives
   * to, whatever the file's id field says
   *
   * @param {Config} config
   * @return {string|null}
   */
  function readRenderedNodeId(config) {
    const filePath = homeDir.joinPath(
      config.getName(),
      'platform',
      'drive',
      'tenderdash',
      'node_key.json',
    );

    try {
      return parseTenderdashNodeKey(fs.readFileSync(filePath, 'utf8')).id;
    } catch {
      return null;
    }
  }

  /**
   * @param {Config} config
   * @return {Object}
   */
  function getSetupState(config) {
    const name = config.getName();

    return getPlatformSetupState(config, {
      defaultConfig: defaultConfigs.has(name) ? defaultConfigs.get(name) : undefined,
      certificateFiles: findCertificateFiles(homeDir, name),
    });
  }

  /**
   * How to finish a ZeroSSL setup that stopped half-way, from the
   * certificate files as they are
   *
   * @param {Object} setupState
   * @param {string} name
   * @param {{ isUsable: boolean, directories: string[] }} certificate
   * @return {string}
   */
  function describeInterruptedSetup(setupState, name, certificate) {
    // Files the gateway cannot serve are not an installed certificate.
    const remedy = describePlatformSetupState(
      certificate.isUsable ? setupState : { ...setupState, markers: [] },
      name,
    );

    if (certificate.directories.length === 0) {
      return remedy;
    }

    // ssl obtain cannot write a file over them.
    return [
      'Docker created directories where the gateway\'s certificate files go, as it does for a'
        + ' file it mounts that is missing. Remove them first:',
      ...certificate.directories.map((directory) => (isShellQuotable(directory)
        ? `  rmdir ${quoteShellArgument(directory)}`
        : `  rmdir ${JSON.stringify(directory)}    (escaped, not to paste)`)),
      remedy,
    ].join('\n');
  }

  /**
   * @param {string} name
   * @param {string} problem
   * @param {string} remedy
   */
  function refuse(name, problem, remedy) {
    const error = new Error(`Cannot start Platform of ${name}: ${problem}.\n${remedy}`);
    error.problem = problem;
    error.remedy = remedy;

    throw error;
  }

  /**
   * Refuse to start Platform of a node whose Platform part is not set up
   *
   * A masternode's Platform runs its registered identity. After
   * `reset --platform --hard` an evonode keeps Platform enabled (testnet) with
   * no node key, so Tenderdash would start on a node_key.json that says
   * "null". A node whose config was saved but whose service files were not
   * rendered would run an older identity.
   *
   * A fullnode needs no registered identity: any key serves it, and dashmate
   * gives it one. After the same reset an Evo fullnode keeps Platform enabled
   * with no certificate, and `update && start` would bring up a gateway that
   * cannot serve TLS, and create the Platform data volumes that setup then
   * refuses as data a reset kept. It is refused while Platform-only setup sees
   * its Platform part as not set up, by the same markers setup checks, and
   * while a ZeroSSL setup that stopped half-way left the gateway without its
   * certificate files. The gateway serves TLS from those files whatever the
   * provider, so a fullnode whose Platform was set up has TLS on or files of
   * its own. Only the presets Platform-only setup serves are refused: it
   * cannot set up others.
   *
   * Every case is refused here, before anything is rendered or started, with
   * the command that sets it right.
   *
   * The error carries `problem` and `remedy`, for callers that word the
   * refusal themselves.
   *
   * @typedef {function} assertPlatformCanStart
   * @param {Config} config
   */
  function assertPlatformCanStart(config) {
    if (!config.get('platform.enable')) {
      return;
    }

    const name = config.getName();
    const flag = renderConfigFlag(name);

    if (!config.get('core.masternode.enable')) {
      if (!PLATFORM_PRESETS.includes(name)) {
        return;
      }

      const setupState = getSetupState(config);

      if (setupState.state === PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED) {
        refuse(name, 'its Platform part is not set up', [
          describePlatformSetupState(setupState, name),
          `To run Core alone until then: dashmate config set platform.enable false ${flag}`,
        ].join('\n'));
      }

      // ZeroSSL saves the answers when it creates the certificate, and a
      // failed verification leaves no certificate behind. Once the issued pair
      // is installed the gateway serves TLS, and only renewal waits.
      if (setupState.state === PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED) {
        const certificate = inspectCertificateFiles(homeDir, name);

        if (!certificate.isUsable) {
          refuse(
            name,
            'its Platform setup did not finish, and the gateway has no certificate',
            describeInterruptedSetup(setupState, name, certificate),
          );
        }
      }

      return;
    }

    const { id, key } = config.get('platform.drive.tenderdash.node');
    const mode = config.get('platform.drive.tenderdash.mode');

    const setupState = getSetupState(config);

    if (key === null) {
      const problem = 'this masternode has no Platform node key';
      const runCoreAlone = `To run Core alone until then: dashmate config set platform.enable false ${flag}`;
      const groupName = config.get('group');

      // The local group's identities, ports and peers belong to the whole
      // group, which `setup local` builds again: local_1 to local_N and
      // local_seed. A config of another name in that group, the refused one
      // or any other, would be removed by the group's reset and not built
      // again.
      if (groupName === PRESET_LOCAL && LOCAL_GROUP_MEMBER_NAME.test(name)
        && findLocalGroupConfigsNotRebuilt(configFile).length === 0) {
        refuse(name, problem, [
          `Set the ${groupName} group up again with:`,
          `  dashmate group reset --group ${groupName} --hard`,
          `  dashmate setup ${groupName}`,
        ].join('\n'));
      }

      // Setup builds only the presets. A hard reset would remove a member of
      // any other group, a config with any other name, or one in the local
      // group with the refused member, entirely, and nothing could create it
      // again, so the remedy is the setting itself.
      // `config set` would take the key on its command line.
      if (groupName || !PLATFORM_PRESETS.includes(name)) {
        refuse(name, problem, [
          ...(setupState.state === PLATFORM_SETUP_STATE.CORE_NOT_SET_UP
            ? [describePlatformSetupState(setupState, name)]
            : []),
          'Put the Tenderdash node key that the masternode\'s registration names into'
            + ' platform.drive.tenderdash.node.key, and its node ID into'
            + ` platform.drive.tenderdash.node.id, by editing ${homeDir.joinPath('config.json')}`
            + ' while no dashmate command runs. Then render the service files with:',
          `  dashmate config render ${flag}`,
          runCoreAlone,
        ].join('\n'));
      }

      refuse(name, problem, [
        describePlatformSetupState(setupState, name),
        runCoreAlone,
      ].join('\n'));
    }

    if (mode !== 'validator') {
      refuse(
        name,
        `this masternode runs Tenderdash in '${mode}' mode, and a masternode must validate`,
        `Set it with:\n  dashmate config set platform.drive.tenderdash.mode validator ${flag}`,
      );
    }

    // Rendering again would not finish it: TLS is still off.
    if (setupState.state === PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED) {
      refuse(
        name,
        'its Platform setup did not finish',
        describeInterruptedSetup(setupState, name, inspectCertificateFiles(homeDir, name)),
      );
    }

    if (readRenderedNodeId(config) !== id) {
      refuse(
        name,
        'the service files do not hold the configured node ID',
        `Render them again with:\n  dashmate config render ${flag}`,
      );
    }
  }

  /**
   * Why the Platform part of a node has to be set up before it is updated, if
   * it has to
   *
   * Besides what stops Platform from starting, this covers an Evo fullnode
   * whose ZeroSSL setup stopped half-way: a certificate obtained now would make
   * Platform setup refuse.
   *
   * @param {Config} config
   * @return {Error|null} with `problem` and `remedy`, and `canStart` when
   *  start lets the node through
   */
  function findPendingPlatformSetup(config) {
    try {
      assertPlatformCanStart(config);
    } catch (e) {
      return e;
    }

    // Platform-only setup serves these presets only.
    if (!config.get('platform.enable') || !PLATFORM_PRESETS.includes(config.getName())) {
      return null;
    }

    const setupState = getSetupState(config);

    if (![PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED, PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED]
      .includes(setupState.state)) {
      return null;
    }

    // What start lets through: a fullnode whose interrupted ZeroSSL setup
    // installed a certificate the gateway serves.
    const error = new Error('Platform setup did not finish');
    error.problem = 'the gateway serves the installed certificate, but nothing renews it until the'
      + ' setup is finished';
    error.remedy = describePlatformSetupState(setupState, config.getName());
    error.canStart = true;

    return error;
  }

  assertPlatformCanStart.findPendingSetup = findPendingPlatformSetup;

  return assertPlatformCanStart;
}

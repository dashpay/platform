import fs from 'fs';
import renderConfigFlag from '../util/renderConfigFlag.js';
import getPlatformSetupState, {
  PLATFORM_SETUP_STATE,
  describePlatformSetupState,
} from './getPlatformSetupState.js';
import { findCertificateFiles } from './getPlatformArtefactsFactory.js';
import parseTenderdashNodeKey from '../tenderdash/parseTenderdashNodeKey.js';
import { PRESET_MAINNET, PRESET_TESTNET } from '../constants.js';

const PLATFORM_PRESETS = [PRESET_MAINNET, PRESET_TESTNET];

/**
 * @param {HomeDir} homeDir
 * @param {DefaultConfigs} defaultConfigs
 * @return {assertPlatformCanStart}
 */
export default function assertPlatformCanStartFactory(homeDir, defaultConfigs) {
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
   * Refuse to start a masternode's Platform without its registered identity
   *
   * After `reset --platform --hard` an evonode keeps Platform enabled (testnet)
   * with no node key, so Tenderdash would start on a node_key.json that says
   * "null". A node whose config was saved but whose service files were not
   * rendered would run an older identity. Both are refused here, before
   * anything is rendered or started, with the command that sets them right.
   *
   * Fullnodes are left alone: any key serves them, and dashmate gives them one.
   *
   * The error carries `problem` and `remedy`, for callers that word the
   * refusal themselves.
   *
   * @typedef {function} assertPlatformCanStart
   * @param {Config} config
   */
  function assertPlatformCanStart(config) {
    if (!config.get('platform.enable') || !config.get('core.masternode.enable')) {
      return;
    }

    const name = config.getName();
    const flag = renderConfigFlag(name);
    const { id, key } = config.get('platform.drive.tenderdash.node');
    const mode = config.get('platform.drive.tenderdash.mode');

    const setupState = getSetupState(config);

    const refuse = (problem, remedy) => {
      const error = new Error(`Cannot start Platform of ${name}: ${problem}.\n${remedy}`);
      error.problem = problem;
      error.remedy = remedy;

      throw error;
    };

    if (key === null) {
      const groupName = config.get('group');

      // A group's identities, ports and peers belong to the whole group, and
      // setting up Platform alone does not support them.
      if (groupName) {
        refuse('this masternode has no Platform node key', [
          `Set the ${groupName} group up again with:`,
          `  dashmate group reset --group ${groupName} --hard`,
          `  dashmate setup ${groupName}`,
        ].join('\n'));
      }

      refuse('this masternode has no Platform node key', [
        describePlatformSetupState(setupState, name),
        `To run Core alone until then: dashmate config set platform.enable false ${flag}`,
      ].join('\n'));
    }

    if (mode !== 'validator') {
      refuse(
        `this masternode runs Tenderdash in '${mode}' mode, and a masternode must validate`,
        `Set it with:\n  dashmate config set platform.drive.tenderdash.mode validator ${flag}`,
      );
    }

    // Rendering again would not finish it: TLS is still off.
    if (setupState.state === PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED) {
      refuse('its Platform setup did not finish', describePlatformSetupState(setupState, name));
    }

    if (readRenderedNodeId(config) !== id) {
      refuse(
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
   * after `reset --platform --hard`: it can start with the key the reset gave
   * it, but a certificate obtained now would make Platform setup refuse.
   *
   * @param {Config} config
   * @return {Error|null} with `problem` and `remedy`
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

    const error = new Error('Platform is not set up');
    error.problem = 'its Platform part is not set up';
    error.remedy = describePlatformSetupState(setupState, config.getName());

    return error;
  }

  assertPlatformCanStart.findPendingSetup = findPendingPlatformSetup;

  return assertPlatformCanStart;
}

import path from 'path';
import renderConfigFlag from '../util/renderConfigFlag.js';
import renderNodeKeyBackup, { NODE_KEY_PATH } from './renderNodeKeyBackup.js';

export const PLATFORM_SETUP_STATE = {
  CORE_NOT_SET_UP: 'CORE_NOT_SET_UP',
  ZEROSSL_INTERRUPTED: 'ZEROSSL_INTERRUPTED',
  PLATFORM_CONFIGURED: 'PLATFORM_CONFIGURED',
  PLATFORM_UNCONFIGURED: 'PLATFORM_UNCONFIGURED',
};

const VOLUME_MARKER_PREFIX = 'Docker volume ';

const PRESET_NAMES = ['mainnet', 'testnet'];

// Both files of the gateway certificate: what ZeroSSL writes last.
const CERTIFICATE_FILE_NAMES = ['bundle.crt', 'private.key'];

/**
 * Whether the Core part of a config was set up
 *
 * Positive evidence only. Every preset is written to config.json on the first
 * run, and migrations change untouched presets too, so differing from the
 * defaults proves nothing. Setup always replaces the placeholder RPC passwords;
 * the dashmate user is the one checked because migrations added the other
 * users later with the placeholder.
 *
 * @param {Config} config
 * @param {Config} [defaultConfig] - the preset's defaults, when it has them
 * @return {boolean}
 */
function isCoreSetUp(config, defaultConfig) {
  if (defaultConfig) {
    if (config.get('network') !== defaultConfig.get('network')) {
      return false;
    }

    const passwordPath = 'core.rpc.users.dashmate.password';
    if (config.get(passwordPath) === defaultConfig.get(passwordPath)) {
      return false;
    }
  }

  if (config.get('core.masternode.enable')) {
    return config.get('core.masternode.operator.privateKey') !== null
      && config.get('externalIp') !== null;
  }

  return true;
}

/**
 * Where the Platform part of a config stands, for a Platform-only setup and
 * the commands that need it done
 *
 * "Unconfigured" is decided by what setup itself writes and by what a Platform
 * that ran leaves behind, not by equality with the defaults. Setup does not own
 * options such as image pins or log levels, so they are not compared: they
 * change for reasons of their own and would refuse untouched configs, while a
 * certificate installed by hand would pass.
 *
 * A fullnode's key does not count: the reset generates one straight away, and
 * nothing registers it.
 *
 * @typedef {function} getPlatformSetupState
 * @param {Config} config
 * @param {Object} [options]
 * @param {Config} [options.defaultConfig] - the preset's defaults
 * @param {string[]} [options.certificateFiles] - gateway certificate files found
 * @param {string[]} [options.dataVolumes] - Platform data volumes that exist
 * @return {{ state: string, markers: string[] }} markers are option paths,
 *  file paths and volumes, never values
 */
export default function getPlatformSetupState(config, {
  defaultConfig,
  certificateFiles = [],
  dataVolumes = [],
} = {}) {
  if (!isCoreSetUp(config, defaultConfig)) {
    return { state: PLATFORM_SETUP_STATE.CORE_NOT_SET_UP, markers: [] };
  }

  const key = config.get('platform.drive.tenderdash.node.key');
  const ssl = config.get('platform.gateway.ssl');
  const zeroSsl = ssl.providerConfigs.zerossl;

  // ZeroSSL saves every answer as soon as it creates a certificate. If the
  // run stops before it ends, the answers stay and TLS stays off. Whether
  // the issued pair reached the disk decides how to finish.
  if (ssl.provider === 'zerossl' && zeroSsl.id !== null && ssl.enabled === false && key !== null) {
    return {
      state: PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED,
      markers: certificateFiles,
      certificateId: zeroSsl.id,
    };
  }

  const markers = [];

  if (config.get('core.masternode.enable') && key !== null) {
    markers.push(NODE_KEY_PATH);
  }

  if (ssl.enabled) {
    markers.push('platform.gateway.ssl.enabled');
  }

  if (zeroSsl.apiKey !== null) {
    markers.push('platform.gateway.ssl.providerConfigs.zerossl.apiKey');
  }

  if (zeroSsl.id !== null) {
    markers.push('platform.gateway.ssl.providerConfigs.zerossl.id');
  }

  markers.push(...certificateFiles);
  markers.push(...dataVolumes.map((name) => `${VOLUME_MARKER_PREFIX}${name}`));

  return {
    state: markers.length > 0
      ? PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED
      : PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED,
    markers,
  };
}

/**
 * What the state means for the operator and what to run
 *
 * @param {{ state: string, markers: string[], certificateId?: string }} setupState
 * @param {string} configName
 * @return {string}
 */
export function describePlatformSetupState({ state, markers, certificateId }, configName) {
  const flag = renderConfigFlag(configName);
  const setupCommand = `dashmate setup --platform ${flag}`;

  switch (state) {
    case PLATFORM_SETUP_STATE.CORE_NOT_SET_UP:
      if (!PRESET_NAMES.includes(configName)) {
        return `The Core part of ${configName} is not set up: a masternode needs`
          + ' core.masternode.operator.privateKey and externalIp.';
      }

      return [
        `Preset ${configName} is not set up yet. Set the whole node up with:`,
        `  dashmate setup ${configName}`,
      ].join('\n');

    case PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED: {
      const isCertificateInstalled = CERTIFICATE_FILE_NAMES
        .every((fileName) => markers.some((marker) => path.basename(marker) === fileName));

      if (isCertificateInstalled) {
        return [
          `An earlier Platform setup of ${configName} stopped after ZeroSSL issued its certificate`,
          'and the certificate was installed. Its answers were saved. To finish the setup:',
          `  dashmate ssl obtain ${flag}`,
          `then continue with 'dashmate update ${flag}' and start the node.`,
        ].join('\n');
      }

      return [
        `An earlier Platform setup of ${configName} stopped after it created a ZeroSSL certificate`,
        'and before the certificate was verified. Its answers were saved, but that certificate',
        'cannot be completed. To finish the setup:',
        '  1. open inbound TCP port 80 to this node',
        `  2. dashmate ssl obtain --force ${flag}    (creates a new certificate)`,
        `  3. cancel the unfinished certificate ${certificateId} in the ZeroSSL dashboard`,
        `then continue with 'dashmate update ${flag}' and start the node.`,
        `('dashmate ssl cleanup ${flag}' would cancel every unfinished certificate of the`,
        'API key\'s account, other nodes\' included.) Do not reset Platform in between: that',
        'removes the certificate ID and the API key from the config.',
      ].join('\n');
    }

    case PLATFORM_SETUP_STATE.PLATFORM_CONFIGURED: {
      const options = markers.filter((marker) => marker.startsWith('platform.'));
      const volumes = markers.filter((marker) => marker.startsWith(VOLUME_MARKER_PREFIX));
      const files = markers.filter((marker) => !options.includes(marker) && !volumes.includes(marker));

      const lines = [
        `Platform of ${configName} was configured before:`,
        ...markers.map((marker) => `  ${marker}`),
      ];

      if (options.length > 0) {
        lines.push(
          `To change single options, use 'dashmate config set <option> <value> ${flag}'.`,
          'To start Platform over instead:',
        );

        // Only a masternode's key counts as configuration, and it is the one
        // the registration names. A backup line without a key would write
        // "null" into the backup.
        const keyBackup = markers.includes(NODE_KEY_PATH)
          ? renderNodeKeyBackup(configName)
          : null;

        if (keyBackup) {
          lines.push(
            '  back the node key up, into a new file only you can read:',
            `    ${keyBackup.backupCommand}`,
          );
        }

        lines.push(
          `  dashmate stop --platform ${flag}    (if Platform is running)`,
          `  dashmate reset --platform --hard ${flag}`,
          `  ${keyBackup ? keyBackup.setupCommand : setupCommand}`,
        );

        return lines.join('\n');
      }

      if (files.length > 0) {
        lines.push(
          'No Platform option refers to these files. If an earlier Platform setup installed them',
          'and then failed, remove them:',
          ...files.map((file) => `  rm '${file}'`),
        );
      }

      if (volumes.length > 0) {
        lines.push(
          'Platform data from before is still there (a reset with --keep-data keeps it).',
          'Remove it with:',
          `  dashmate reset --platform ${flag}`,
        );
      }

      lines.push(`Then set Platform up with:\n  ${setupCommand}`);

      return lines.join('\n');
    }

    case PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED:
      return [
        `Platform of ${configName} is not set up. Set it up with:`,
        `  ${setupCommand}`,
      ].join('\n');

    default:
      throw new Error(`Unknown Platform setup state ${state}`);
  }
}

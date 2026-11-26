import fs from 'fs';
import { Listr } from 'listr2';

import { Args, Flags } from '@oclif/core';

import chalk from 'chalk';
import BaseCommand from '../oclif/command/BaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';
import printUnwrapped from '../oclif/errors/printUnwrapped.js';
import Config from '../config/Config.js';
import { findCertificateFiles } from '../config/getPlatformArtefactsFactory.js';
import getPlatformSetupState, {
  PLATFORM_SETUP_STATE,
  describePlatformSetupState,
} from '../config/getPlatformSetupState.js';
import parseTenderdashNodeKey from '../tenderdash/parseTenderdashNodeKey.js';
import readSecretInput, { getReadableByOthersWarning } from '../util/readSecretInput.js';
import isInteractiveSession from '../util/isInteractiveSession.js';
import promptOrThrow from '../util/promptOrThrow.js';
import renderConfigFlag from '../util/renderConfigFlag.js';
import validateIPv4 from '../listr/prompts/validators/validateIPv4.js';
import { readCertificateFilePair } from '../listr/prompts/validators/validateCertificateFilePaths.js';
import createPortValidators, { findPortCollision } from '../listr/prompts/validators/createPortValidators.js';
import { getCertificateProviders } from '../listr/tasks/setup/regular/configureSSLCertificateTaskFactory.js';

import {
  NODE_TYPE_FULLNODE,
  NODE_TYPE_MASTERNODE,
  PRESET_LOCAL,
  PRESET_MAINNET,
  PRESET_TESTNET,
  PRESETS,
  SSL_PROVIDERS,
} from '../constants.js';

const PLATFORM_PRESETS = [PRESET_MAINNET, PRESET_TESTNET];

// Flags that answer full setup's questions about the local network and Core.
const FULL_SETUP_FLAGS = ['node-count', 'miner-interval', 'debug-logs', 'enable-tor'];

// Flags that answer Platform-only setup's questions.
const PLATFORM_SETUP_FLAGS = [
  'config', 'non-interactive', 'platform-node-key-file', 'platform-p2p-port',
  'platform-http-port', 'external-ip', 'ssl-provider', 'zerossl-api-key-file', 'ssl-chain-file',
  'ssl-private-key-file', 'accept-unmet-requirements',
];

const SECRET_FILE_FLAGS = ['platform-node-key-file', 'zerossl-api-key-file'];

export default class SetupCommand extends BaseCommand {
  // Reconfigures the node: changes configuration repeatedly while doing long,
  // partly irreversible work, so it holds the config lock for its whole run.
  static mutatesConfig = true;

  static description = 'Set up a new Dash node, or the Platform part of a node whose Core part is set up';

  static args = {
    preset: Args.string(
      {
        name: 'preset',
        required: false,
        description: 'Node configuration preset',
        options: Object.values(PRESETS),
      },
    ),
  };

  static flags = {
    'debug-logs': Flags.boolean({ char: 'd', description: 'enable debug logs', allowNo: true }),
    'node-count': Flags.integer({ char: 'c', description: 'number of nodes to setup' }),
    'miner-interval': Flags.string({ char: 'm', description: 'interval between blocks' }),
    'enable-tor': Flags.boolean({ description: 'run a Tor sidecar so Core reaches onion peers and publishes an onion service (default: enabled)', allowNo: true }),

    platform: Flags.boolean({
      char: 'p',
      description: 'set up only Platform, on a mainnet or testnet node whose Core part is set up'
        + ' and whose Platform part is not (after `reset --platform --hard`)',
      default: false,
    }),
    config: Flags.string({ description: 'with --platform: the preset to set up, instead of PRESET' }),
    'non-interactive': Flags.boolean({
      description: 'with --platform: never prompt; every answer comes from a flag. Also'
        + ' DASHMATE_NON_INTERACTIVE, CI, and a stdin or stdout that is not a terminal',
      default: false,
    }),
    'platform-node-key-file': Flags.string({
      description: 'with --platform: file holding the Tenderdash node key (base64, or a'
        + ' node_key.json), or - for stdin',
    }),
    'platform-p2p-port': Flags.integer({ description: 'with --platform: Platform P2P port' }),
    'platform-http-port': Flags.integer({ description: 'with --platform: Platform HTTP port' }),
    'external-ip': Flags.string({ description: 'with --platform: external IPv4 address, for a fullnode that has none' }),
    'ssl-provider': Flags.string({
      description: 'with --platform: how to get the gateway certificate',
      options: Object.values(SSL_PROVIDERS),
    }),
    'zerossl-api-key-file': Flags.string({ description: 'with --platform: file holding the ZeroSSL API key, or - for stdin' }),
    'ssl-chain-file': Flags.string({ description: 'with --platform: certificate chain file, for --ssl-provider file' }),
    'ssl-private-key-file': Flags.string({ description: 'with --platform: certificate private key file, for --ssl-provider file' }),
    'accept-unmet-requirements': Flags.boolean({
      description: 'with --platform: proceed although the system does not meet the requirements',
      default: false,
    }),

    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @param {setupLocalPresetTask} setupLocalPresetTask
   * @param {setupRegularPresetTask} setupRegularPresetTask
   * @param {DockerCompose} dockerCompose
   * @param {DefaultConfigs} defaultConfigs
   * @param {setupPlatformTask} setupPlatformTask
   * @param {getPlatformArtefacts} getPlatformArtefacts
   * @param {HomeDir} homeDir
   * @param {StartedContainers} startedContainers
   * @param {Docker} docker
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    configFile,
    setupLocalPresetTask,
    setupRegularPresetTask,
    dockerCompose,
    defaultConfigs,
    setupPlatformTask,
    getPlatformArtefacts,
    homeDir,
    startedContainers,
    docker,
  ) {
    if (flags.platform) {
      return this.runPlatformSetup(args, flags, {
        configFile,
        dockerCompose,
        defaultConfigs,
        setupPlatformTask,
        getPlatformArtefacts,
        homeDir,
        startedContainers,
        docker,
      });
    }

    const misplaced = PLATFORM_SETUP_FLAGS.filter((name) => (
      flags[name] !== undefined && flags[name] !== false
    ));

    if (misplaced.length > 0) {
      throw new Error(`${misplaced.map((name) => `--${name}`).join(', ')}`
        + ` ${misplaced.length === 1 ? 'works' : 'work'} only with --platform`);
    }

    return this.runFullSetup(args, flags, {
      configFile,
      setupLocalPresetTask,
      setupRegularPresetTask,
      defaultConfigs,
    });
  }

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Object} dependencies
   * @return {Promise<void>}
   */
  async runFullSetup(
    { preset },
    {
      'node-count': nodeCount,
      'debug-logs': debugLogs,
      'miner-interval': minerInterval,
      'enable-tor': enableTor,
      verbose: isVerbose,
    },
    {
      configFile,
      setupLocalPresetTask,
      setupRegularPresetTask,
      defaultConfigs,
    },
  ) {
    if (nodeCount !== null && (nodeCount < 3)) {
      throw new Error('node-count flag should be not less than 3');
    }

    const tasks = new Listr(
      [
        {
          title: 'Configuration preset',
          task: async (ctx, task) => {
            if (ctx.preset === undefined) {
              ctx.preset = await task.prompt([
                {
                  type: 'select',
                  header: `  Dashmate provides three default configuration presets:

    mainnet - Run a node connected to the Dash main network
    testnet - Run a node connected to the Dash test network
    local   - Run a full network environment on your machine for local development\n`,
                  message: 'Select preset',
                  choices: Object.values(PRESETS),
                  initial: PRESET_MAINNET,
                },
              ]);
            }

            let isAlreadyConfigured;
            if (ctx.preset === PRESET_LOCAL) {
              isAlreadyConfigured = configFile.isGroupExists(ctx.preset);
            } else {
              const defaultConfig = defaultConfigs.get(ctx.preset);

              isAlreadyConfigured = !configFile.getConfig(ctx.preset).isEqual(defaultConfig);
            }

            if (isAlreadyConfigured) {
              // eslint-disable-next-line no-param-reassign
              task.output = ctx.preset === PRESET_LOCAL
                ? chalk`Preset {bold ${ctx.preset}} already configured.

  To set up a node with this preset from scratch use {bold.cyanBright dashmate group reset --group ${ctx.preset} --hard}.
  Previous data and configuration for this preset will be lost.`
                : SetupCommand.renderAlreadyConfigured(configFile.getConfig(ctx.preset), defaultConfigs);

              throw new Error(`Preset ${ctx.preset} already configured`);
            } else {
            // eslint-disable-next-line no-param-reassign
              task.output = ctx.preset;
            }
          },
          options: {
            persistentOutput: true,
            showErrorMessage: false,
          },
        },
        {
          task: (ctx) => {
            if (ctx.preset === PRESET_LOCAL) {
              return setupLocalPresetTask();
            }

            return setupRegularPresetTask();
          },
        },
      ],
      {
        concurrent: false,
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    if (!isVerbose) { // TODO: We need to print it only with default renderer
      // eslint-disable-next-line import-x/extensions
      const { begoo } = await import('begoo/index.js'); // don't remove index!

      const welcomeText = begoo(
        chalk`Hello! I'm your {bold.cyanBright Dash} mate!

I will assist you with setting up a Dash node on mainnet or testnet. I can also help you set up a development network on your local system.`,
        { maxLength: 45 },
      );

      // eslint-disable-next-line no-console
      console.log(welcomeText);
    }

    try {
      await tasks.run({
        preset,
        nodeCount,
        debugLogs,
        minerInterval,
        enableTor,
        isVerbose,
        // Full setup asks the operator a question at every step, so it cannot
        // run unattended and states this rather than detecting it. The tasks
        // read it to decide whether they may prompt.
        interactive: true,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }

  /**
   * What full setup says about a preset that is already configured
   *
   * When Platform is not set up, Platform-only setup is offered next to the
   * usual advice: after `reset --platform --hard` it is the way back, and a
   * full reset would take the Core part with it. It is not offered in place of
   * that advice, because a Regular masternode or a Core-only fullnode looks
   * the same and may want neither.
   *
   * @param {Config} config
   * @param {DefaultConfigs} defaultConfigs
   * @return {string}
   */
  static renderAlreadyConfigured(config, defaultConfigs) {
    const name = config.getName();

    const { state } = getPlatformSetupState(config, { defaultConfig: defaultConfigs.get(name) });

    const platformHint = state === PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED
      ? chalk`

  If its Platform part was reset with {bold.cyanBright dashmate reset --platform --hard}, or to add Platform
  to an Evo node, set up only Platform, keeping Core as it is, with
  {bold.cyanBright dashmate setup --platform ${renderConfigFlag(name)}}.`
      : '';

    return chalk`Preset {bold ${name}} already configured.

  To set up a node with this preset from scratch use {bold.cyanBright dashmate reset ${renderConfigFlag(name)} --hard}.
  Previous data and configuration for this preset will be lost.

  If you want to keep the existing data and configuration, please use the {bold.cyanBright dashmate config create}
  command to create a new configuration for this preset.${platformHint}`;
  }

  /**
   * Set up the Platform part of a preset whose Core part is set up
   *
   * Everything that can refuse without the network refuses here, before Core
   * is started or a certificate provider is contacted: the preconditions, and
   * every answer an unattended run needs.
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {Object} dependencies
   * @return {Promise<void>}
   */
  async runPlatformSetup(args, flags, dependencies) {
    // A prompt that leaks past the interactivity guard neither throws nor
    // settles, and the process would exit 0 with nothing done.
    process.exitCode = 1;

    let context;
    try {
      context = await SetupCommand.preparePlatformSetup(args, flags, dependencies);
    } catch (e) {
      // Refusals give commands to copy, which oclif would hard-wrap.
      throw printUnwrapped(e);
    }

    const tasks = new Listr([
      {
        title: `Set up Platform of ${context.preset}`,
        task: () => dependencies.setupPlatformTask(),
      },
    ], {
      renderer: flags.verbose ? 'verbose' : 'default',
      rendererOptions: {
        showTimer: flags.verbose,
        // Errors explain what to do next on several lines; truncating them to
        // the terminal width would cut the commands off.
        formatOutput: 'wrap',
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
        removeEmptyLines: false,
      },
    });

    let failure;
    try {
      await tasks.run(context);
    } catch (e) {
      failure = e;
    }

    await SetupCommand.stopStartedContainers(dependencies, flags);

    if (failure) {
      SetupCommand.reportPlatformSetupFailure(context, dependencies.homeDir);

      throw new MuteOneLineError(failure);
    }

    process.exitCode = 0;

    return undefined;
  }

  /**
   * Stop the containers this command started, the way `dashmate stop` stops
   * them
   *
   * The command's cleanup on exit force-removes the containers it started, and
   * the Core that setup starts runs on the node's chain database. Docker sends
   * each SIGTERM and waits for the stop grace period of its service instead.
   * Every container the command started counts, so a Core whose container
   * Compose created and which then never answered is stopped this way too. A
   * Core that was running before setup is not among them. If stopping fails,
   * that cleanup still removes the containers.
   *
   * @param {Object} dependencies
   * @param {StartedContainers} dependencies.startedContainers
   * @param {Docker} dependencies.docker
   * @param {Object} flags
   * @return {Promise<void>}
   */
  static async stopStartedContainers({ startedContainers, docker }, flags) {
    try {
      const containers = (await Promise.all(startedContainers.getContainers().map(async (id) => {
        const container = docker.getContainer(id);

        try {
          const { State: state } = await container.inspect();

          return state.Running ? container : null;
        } catch (e) {
          // Removed already
          if (e.statusCode === 404) {
            return null;
          }

          throw e;
        }
      }))).filter((container) => container !== null);

      if (containers.length === 0) {
        return;
      }

      await new Listr([{
        title: 'Stop the containers setup started',
        task: () => Promise.all(containers.map((container) => container.stop())),
      }], {
        renderer: flags.verbose ? 'verbose' : 'default',
        rendererOptions: { clearOutput: false, collapse: false },
      }).run();
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(`Could not stop the containers setup started gracefully (${e.message});`
        + ' they are stopped by force now.');
    }
  }

  /**
   * Check the preconditions and the answers given in advance, and build the
   * context the setup task runs with
   *
   * @param {Object} args
   * @param {Object} flags
   * @param {Object} dependencies
   * @return {Promise<Object>}
   */
  static async preparePlatformSetup(args, flags, {
    configFile,
    dockerCompose,
    defaultConfigs,
    getPlatformArtefacts,
    homeDir,
  }) {
    const fullSetupFlags = FULL_SETUP_FLAGS.filter((name) => flags[name] !== undefined);
    if (fullSetupFlags.length > 0) {
      throw new Error(`${fullSetupFlags.map((name) => `--${name}`).join(', ')} cannot be used`
        + ' with --platform');
    }

    const stdinFlags = [...SECRET_FILE_FLAGS, 'ssl-chain-file', 'ssl-private-key-file']
      .filter((name) => flags[name] === '-');
    if (stdinFlags.length > 1) {
      throw new Error(`Only one flag can read stdin; ${stdinFlags.map((name) => `--${name}`).join(' and ')} both ask for it`);
    }
    if (stdinFlags.some((name) => !SECRET_FILE_FLAGS.includes(name))) {
      throw new Error(`--${stdinFlags[0]} needs a file path`);
    }

    // Reading stdin leaves no terminal to answer prompts, which
    // isInteractiveSession sees too.
    const interactive = isInteractiveSession({ flags });

    const preset = await SetupCommand.resolvePlatformPreset(args.preset, flags.config, {
      configFile,
      interactive,
    });

    const config = configFile.getConfig(preset);
    const defaultConfig = defaultConfigs.get(preset);
    const isMasternode = config.get('core.masternode.enable') === true;

    const refuse = (message) => {
      throw new Error(`Cannot set up Platform of ${preset}.\n${message}`);
    };

    // What the config and its certificate files show: Core not set up, an
    // interrupted setup, or options a Platform setup or certificate command
    // wrote.
    let setupState = getPlatformSetupState(config, {
      defaultConfig,
      certificateFiles: findCertificateFiles(homeDir, preset),
    });
    if (setupState.state !== PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED) {
      refuse(describePlatformSetupState(setupState, preset));
    }

    let artefacts;
    try {
      artefacts = await getPlatformArtefacts(config);
    } catch (e) {
      refuse(`Cannot check its Platform services and data: ${e.message}`);
    }

    setupState = getPlatformSetupState(config, { defaultConfig, ...artefacts });
    if (setupState.state !== PLATFORM_SETUP_STATE.PLATFORM_UNCONFIGURED) {
      refuse(describePlatformSetupState(setupState, preset));
    }

    if (artefacts.activeServices.length > 0) {
      refuse(`Platform services are running: ${artefacts.activeServices.join(', ')}. Stop them with:\n`
        + `  dashmate stop --platform ${renderConfigFlag(preset)}`);
    }

    const answers = await SetupCommand.readPlatformAnswers(flags, {
      config,
      defaultConfigs,
      preset,
      isMasternode,
      interactive,
    });

    // Answers are applied to a copy; the command saves it only if setup
    // finishes, so a refusal or an interrupted run leaves config.json as it is.
    const setupConfig = new Config(config.getName(), config.getStoredOptions());
    setupConfig.set('platform.enable', true);
    setupConfig.set('platform.drive.tenderdash.mode', isMasternode ? 'validator' : 'full');

    return {
      ...answers,
      preset,
      config: setupConfig,
      storedConfig: config,
      isMasternode,
      nodeType: isMasternode ? NODE_TYPE_MASTERNODE : NODE_TYPE_FULLNODE,
      isHP: true,
      interactive,
      wasPlatformEnabled: config.get('platform.enable'),
      isCoreRunning: await dockerCompose.isServiceRunning(config, 'core'),
      // `ssl obtain` would record a provider and enable TLS behind setup's
      // back, and this command would then refuse to run again.
      certificateRetryCommand: `dashmate setup --platform ${renderConfigFlag(preset)}`,
    };
  }

  /**
   * @param {string|undefined} presetArg
   * @param {string|undefined} configFlag
   * @param {Object} options
   * @param {ConfigFile} options.configFile
   * @param {boolean} options.interactive
   * @return {Promise<string>}
   */
  static async resolvePlatformPreset(presetArg, configFlag, { configFile, interactive }) {
    if (presetArg !== undefined && configFlag !== undefined && presetArg !== configFlag) {
      throw new Error(`PRESET ${presetArg} and --config ${configFlag} name different configs`);
    }

    let preset = presetArg ?? configFlag;

    if (preset === undefined) {
      const defaultName = configFile.getDefaultConfigName();

      if (PLATFORM_PRESETS.includes(defaultName)) {
        preset = defaultName;
      } else if (interactive) {
        await new Listr([{
          title: 'Configuration preset',
          task: async (ctx, task) => {
            preset = await promptOrThrow(task, {
              type: 'select',
              message: 'Which preset should Platform be set up for?',
              choices: PLATFORM_PRESETS,
              initial: PRESET_MAINNET,
            }, { interactive });
          },
        }]).run();
      } else {
        throw new Error('Name the preset: dashmate setup --platform mainnet, or testnet');
      }
    }

    if (preset === PRESET_LOCAL) {
      throw new Error('Platform-only setup does not support the local group: its identities, ports,'
        + ' chain and peers belong to the whole group. Set it up again with:\n'
        + '  dashmate group reset --group local --hard\n'
        + '  dashmate setup local');
    }

    if (!PLATFORM_PRESETS.includes(preset)) {
      throw new Error(`Platform-only setup supports the mainnet and testnet presets, not ${preset}`);
    }

    return preset;
  }

  /**
   * Read and check every answer given by flag, and require the ones an
   * unattended run cannot do without
   *
   * @param {Object} flags
   * @param {Object} options
   * @return {Promise<Object>} the answers, as context fields
   */
  static async readPlatformAnswers(flags, {
    config,
    defaultConfigs,
    preset,
    isMasternode,
    interactive,
  }) {
    const nodeType = isMasternode ? NODE_TYPE_MASTERNODE : NODE_TYPE_FULLNODE;
    const answers = {
      acceptUnmetSystemRequirements: flags['accept-unmet-requirements'] === true,
    };

    const required = [];
    const warnings = [];

    const provider = flags['ssl-provider'];
    if (provider !== undefined) {
      const providers = getCertificateProviders(nodeType);

      if (!providers.includes(provider)) {
        throw new Error(`--ssl-provider ${provider} cannot be used for a ${nodeType}: update refuses`
          + ` a self-signed certificate on any masternode. Choose one of: ${providers.join(', ')}`);
      }

      answers.certificateProvider = provider;
    } else if (!interactive) {
      required.push('--ssl-provider');
    }

    if (flags['zerossl-api-key-file'] !== undefined) {
      if (provider !== SSL_PROVIDERS.ZEROSSL) {
        throw new Error('--zerossl-api-key-file only works with --ssl-provider zerossl');
      }

      const { content, warnings: fileWarnings } = await readSecretInput(
        flags['zerossl-api-key-file'],
        { flag: '--zerossl-api-key-file' },
      );
      warnings.push(...fileWarnings);

      // Whatever the file holds is sent to ZeroSSL, so a wrong path (the node
      // key file, say) must not get that far.
      const apiKey = content.trim();
      if (!/^[A-Za-z0-9_-]{32,256}$/.test(apiKey)) {
        throw new Error('--zerossl-api-key-file does not hold a ZeroSSL API key: one token of at'
          + ' least 32 letters and digits');
      }

      answers.zeroSslApiKey = apiKey;
    } else if (provider === SSL_PROVIDERS.ZEROSSL && !interactive) {
      required.push('--zerossl-api-key-file');
    }

    const chainFilePath = flags['ssl-chain-file'];
    const privateFilePath = flags['ssl-private-key-file'];
    if (chainFilePath !== undefined || privateFilePath !== undefined) {
      if (provider !== SSL_PROVIDERS.FILE) {
        throw new Error('--ssl-chain-file and --ssl-private-key-file only work with --ssl-provider file');
      }

      if (chainFilePath === undefined || privateFilePath === undefined) {
        throw new Error('--ssl-provider file needs both --ssl-chain-file and --ssl-private-key-file');
      }

      // Installed after Core has synced, which can take hours: what is
      // installed then is what is checked now, not the files as they are by
      // that time.
      try {
        answers.validatedCertificatePair = readCertificateFilePair({ chainFilePath, privateFilePath });
      } catch (e) {
        throw new Error(`--ssl-chain-file, --ssl-private-key-file: ${e.message}`);
      }

      const keyWarning = getReadableByOthersWarning(
        privateFilePath,
        '--ssl-private-key-file',
        answers.validatedCertificatePair.privateKeyStats,
      );
      if (keyWarning) {
        warnings.push(keyWarning);
      }

      answers.fileCertificateProviderForm = { chainFilePath, privateFilePath };
    } else if (provider === SSL_PROVIDERS.FILE && !interactive) {
      required.push('--ssl-chain-file', '--ssl-private-key-file');
    }

    if (flags['platform-node-key-file'] !== undefined) {
      const { content, warnings: fileWarnings } = await readSecretInput(
        flags['platform-node-key-file'],
        { flag: '--platform-node-key-file' },
      );
      warnings.push(...fileWarnings);

      try {
        answers.platformNodeKey = parseTenderdashNodeKey(content);
      } catch (e) {
        throw new Error(`--platform-node-key-file: ${e.message}`);
      }
    } else if (isMasternode && !interactive) {
      required.push('--platform-node-key-file');
    }

    const externalIp = config.get('externalIp');
    if (flags['external-ip'] !== undefined) {
      if (!validateIPv4(flags['external-ip'])) {
        throw new Error('--external-ip must be an IPv4 address');
      }

      // Setup writes externalIp only when it is unset. An equal answer lets
      // the same script run on any node.
      if (externalIp !== null && flags['external-ip'] !== externalIp) {
        throw new Error(`--external-ip ${flags['external-ip']} is not this node's external IP`
          + ` ${externalIp}. Platform setup does not change it; use 'dashmate config set externalIp`
          + ` <ip> ${renderConfigFlag(preset)}' first if it changed`);
      }

      answers.externalIpAnswer = flags['external-ip'];
    } else if (!isMasternode && externalIp === null && !interactive) {
      required.push('--external-ip');
    }

    const { validatePlatformP2PPort, validatePlatformHTTPPort } = createPortValidators(
      defaultConfigs,
      preset,
    );
    const mainnet = defaultConfigs.get(PRESET_MAINNET);

    [
      ['platform-p2p-port', 'platformP2PPort', validatePlatformP2PPort, 'platform.drive.tenderdash.p2p.port'],
      ['platform-http-port', 'platformHttpPort', validatePlatformHTTPPort, 'platform.gateway.listeners.dapiAndDrive.port'],
    ].forEach(([name, field, validate, path]) => {
      const port = flags[name];

      if (port === undefined) {
        return;
      }

      const validity = validate(String(port));
      if (validity !== true) {
        throw new Error(`--${name} ${port}: ${typeof validity === 'string' ? validity : 'not a valid port'}`);
      }

      // Mainnet ports are fixed for fullnodes, as in full setup. A masternode's
      // come from its registration and are checked against it.
      if (!isMasternode && preset === PRESET_MAINNET && port !== mainnet.get(path)) {
        throw new Error(`--${name} ${port}: a mainnet fullnode uses port ${mainnet.get(path)}`);
      }

      answers[field] = port;
    });

    // A masternode's ports are the registered ones, checked when they are read.
    if (!isMasternode && (answers.platformP2PPort !== undefined
      || answers.platformHttpPort !== undefined)) {
      const collision = findPortCollision({
        core: config.get('core.p2p.port'),
        p2p: answers.platformP2PPort ?? config.get('platform.drive.tenderdash.p2p.port'),
        http: answers.platformHttpPort ?? config.get('platform.gateway.listeners.dapiAndDrive.port'),
      });

      if (collision) {
        throw new Error(`Cannot use these Platform ports: ${collision}`);
      }
    }

    if (required.length > 0) {
      throw new Error(`Platform setup is not interactive here, so it needs ${required.join(', ')}.`
        + ' See dashmate setup --help');
    }

    // eslint-disable-next-line no-console
    warnings.forEach((warning) => console.warn(warning));

    return answers;
  }

  /**
   * Say what state a failed run left behind, and clear what would block a
   * second run
   *
   * Once config.json is written, only rendering the service files is left, and
   * the certificate it refers to stays. Before that, only ZeroSSL writes
   * config.json, recording the certificate it created, so once it has, its
   * files stay and the message says how to finish. Any other certificate this
   * run installed is removed:
   * no Platform option refers to it, and setup would refuse to run again over
   * it. The preconditions proved neither file was there before the run. A
   * Let's Encrypt certificate stays in lego's own directory, where the next
   * run finds it.
   *
   * @param {Object} ctx
   * @param {HomeDir} homeDir
   */
  static reportPlatformSetupFailure(ctx, homeDir) {
    const flag = renderConfigFlag(ctx.preset);
    const certificateFiles = findCertificateFiles(homeDir, ctx.preset);

    let message;

    if (ctx.isConfigSaved) {
      message = [
        'Platform setup saved the configuration, and then could not render the service files.',
        'Render them with:',
        `  dashmate config render ${flag}`,
        `then continue with 'dashmate update ${flag}' and start the node.`,
      ].join('\n');
    } else if (ctx.isZeroSslCertificateRecorded) {
      message = describePlatformSetupState({
        state: PLATFORM_SETUP_STATE.ZEROSSL_INTERRUPTED,
        markers: certificateFiles,
        certificateId: ctx.config.get('platform.gateway.ssl.providerConfigs.zerossl.id'),
      }, ctx.preset);
    } else if (ctx.isCertificateStepStarted) {
      const lines = ['Platform setup did not finish, and nothing was saved.'];

      // Created, and then config.json could not be written.
      if (ctx.createdCertificate) {
        lines.push(`ZeroSSL created certificate ${ctx.certificate?.id}, which is recorded nowhere:`
          + ' cancel it in the ZeroSSL dashboard.');
      }

      const leftFiles = certificateFiles.filter((filePath) => {
        try {
          fs.rmSync(filePath, { force: true });

          return false;
        } catch {
          return true;
        }
      });

      if (certificateFiles.length > leftFiles.length) {
        lines.push('The certificate files it installed were removed.');
      }

      if (leftFiles.length > 0) {
        lines.push(
          'Remove the certificate files it installed, or setup refuses to run again:',
          ...leftFiles.map((filePath) => `  rm '${filePath}'`),
        );
      }

      lines.push(
        'Fix the cause above and run it again:',
        `  dashmate setup --platform ${flag}`,
      );

      if (ctx.certificateProvider === SSL_PROVIDERS.LETSENCRYPT) {
        lines.push(
          'A Let\'s Encrypt certificate already issued for this node is reused then. Do not reset',
          'Platform in between: that deletes it, and asking again spends another of the weekly',
          'issuances.',
        );
      }

      lines.push('Do not run dashmate ssl obtain for this node before setup finished: it records a'
        + ' provider, and setup then refuses to run.');

      message = lines.join('\n');
    } else {
      return;
    }

    // eslint-disable-next-line no-console
    console.error(`\n${message}\n`);
  }
}

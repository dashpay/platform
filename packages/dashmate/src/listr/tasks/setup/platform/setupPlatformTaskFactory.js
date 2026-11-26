import { Listr } from 'listr2';
import lodash from 'lodash';
import { Observable } from 'rxjs';
import { publicIpv4 } from 'public-ip';

import Config from '../../../../config/Config.js';
import { PLATFORM_ONLY_RENDER_SCOPE, PRESET_MAINNET } from '../../../../constants.js';
import { REGISTRATION_STATUS } from '../../../../core/readPlatformRegistration.js';
import parseTenderdashNodeKey from '../../../../tenderdash/parseTenderdashNodeKey.js';
import generateTenderdashNodeKey from '../../../../tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../../tenderdash/deriveTenderdashNodeId.js';
import promptOrThrow from '../../../../util/promptOrThrow.js';
import renderConfigFlag from '../../../../util/renderConfigFlag.js';
import wait from '../../../../util/wait.js';
import validateIPv4 from '../../../prompts/validators/validateIPv4.js';
import createPortValidators, { findPortCollision } from '../../../prompts/validators/createPortValidators.js';

const P2P_PORT_PATH = 'platform.drive.tenderdash.p2p.port';
const HTTP_PORT_PATH = 'platform.gateway.listeners.dapiAndDrive.port';

/**
 * @param {string} value
 * @return {boolean|string}
 */
function validateNodeKeyAnswer(value) {
  try {
    parseTenderdashNodeKey(value);

    return true;
  } catch (e) {
    return e.message;
  }
}

/**
 * @param {ConfigFile} configFile
 * @param {DockerCompose} dockerCompose
 * @param {startCore} startCore
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @param {waitForCoreSync} waitForCoreSync
 * @param {readPlatformRegistration} readPlatformRegistration
 * @param {verifySystemRequirementsTask} verifySystemRequirementsTask
 * @param {configureSSLCertificateTask} configureSSLCertificateTask
 * @param {getPlatformArtefacts} getPlatformArtefacts
 * @param {DefaultConfigs} defaultConfigs
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {writeConfigTemplates} writeConfigTemplates
 * @return {setupPlatformTask}
 */
export default function setupPlatformTaskFactory(
  configFile,
  dockerCompose,
  startCore,
  createRpcClient,
  getConnectionHost,
  waitForCoreSync,
  readPlatformRegistration,
  verifySystemRequirementsTask,
  configureSSLCertificateTask,
  getPlatformArtefacts,
  defaultConfigs,
  configFileRepository,
  writeConfigTemplates,
) {
  /**
   * The answer for the Platform node key of an evonode: the registered one only
   *
   * Tenderdash authenticates node IDs when validators dial each other, so no
   * other key can work, and there is no way past this check.
   *
   * @param {Object} ctx
   * @param {Object} task
   * @return {Promise<{ key: string, id: string }>}
   */
  async function getRegisteredNodeKey(ctx, task) {
    const { platformNodeId: registeredId, proTxHash } = ctx.registration;

    let nodeKey = ctx.platformNodeKey;

    if (!nodeKey) {
      const answer = await promptOrThrow(task, {
        type: 'password',
        header: `  Enter the Platform node key of this evonode: the key whose node ID is
  ${registeredId}, as its registration records it.

  It is in your backup of the key or of config.json
  (platform.drive.tenderdash.node.key), or in DMT. To give it from a file
  instead of typing it, run setup with --platform-node-key-file <file>.\n`,
        message: 'Platform node key',
        validate: (value) => {
          const validity = validateNodeKeyAnswer(value);

          if (validity !== true) {
            return validity;
          }

          const { id } = parseTenderdashNodeKey(value);

          return id === registeredId
            || `This key's node ID is ${id}, and the registered one is ${registeredId}`;
        },
      }, { interactive: ctx.interactive });

      nodeKey = parseTenderdashNodeKey(answer);
    }

    if (nodeKey.id !== registeredId) {
      throw new Error([
        `The node key has node ID ${nodeKey.id}, and the registration of this masternode`
          + ` (${proTxHash}) has node ID ${registeredId}.`,
        'Tenderdash authenticates node IDs when validators connect, so only the registered key'
          + ' works.',
        'If you just submitted a ProUpServTx with a new key, wait until it is mined, then run'
          + ' this again.',
      ].join('\n'));
    }

    return nodeKey;
  }

  /**
   * The answer for the Platform node key of an Evo fullnode, which no
   * registration binds
   *
   * A stored key is kept unless the operator says otherwise; a new one is never
   * generated over it implicitly.
   *
   * @param {Object} ctx
   * @param {Object} task
   * @return {Promise<{ key: string, id: string }>}
   */
  async function getFullnodeNodeKey(ctx, task) {
    if (ctx.platformNodeKey) {
      return ctx.platformNodeKey;
    }

    const storedKey = ctx.config.get('platform.drive.tenderdash.node.key');
    const stored = storedKey === null ? null : {
      key: storedKey,
      id: ctx.config.get('platform.drive.tenderdash.node.id') ?? deriveTenderdashNodeId(storedKey),
    };

    const generate = () => {
      const key = generateTenderdashNodeKey();

      return { key, id: deriveTenderdashNodeId(key) };
    };

    if (!ctx.interactive) {
      return stored ?? generate();
    }

    const choices = [];
    if (stored) {
      choices.push({ name: 'keep', message: `Keep the stored key (node ID ${stored.id})` });
    }
    choices.push(
      { name: 'generate', message: 'Generate a new key' },
      { name: 'enter', message: 'Enter a key' },
    );

    const choice = await promptOrThrow(task, {
      type: 'select',
      header: `  The Platform node key identifies this node to its Platform peers. An Evo
  fullnode is not registered, so any Ed25519 key serves.\n`,
      message: 'Platform node key',
      choices,
      initial: stored ? 'keep' : 'generate',
    }, { interactive: ctx.interactive });

    if (choice === 'keep') {
      return stored;
    }

    if (choice === 'generate') {
      return generate();
    }

    const answer = await promptOrThrow(task, {
      type: 'password',
      message: 'Platform node key',
      validate: validateNodeKeyAnswer,
    }, { interactive: ctx.interactive });

    return parseTenderdashNodeKey(answer);
  }

  /**
   * @param {Object} ctx
   * @param {Object} task
   * @return {Promise<{ p2p: number, http: number }>}
   */
  async function getFullnodeEndpoints(ctx, task) {
    // Mainnet ports are fixed, as in full setup; the flags were checked
    // against them before anything ran.
    if (ctx.preset === PRESET_MAINNET) {
      const mainnet = defaultConfigs.get(PRESET_MAINNET);

      return { p2p: mainnet.get(P2P_PORT_PATH), http: mainnet.get(HTTP_PORT_PATH) };
    }

    const isAnswered = ctx.platformP2PPort !== undefined || ctx.platformHttpPort !== undefined;

    if (isAnswered || !ctx.interactive) {
      return {
        p2p: ctx.platformP2PPort ?? ctx.config.get(P2P_PORT_PATH),
        http: ctx.platformHttpPort ?? ctx.config.get(HTTP_PORT_PATH),
      };
    }

    const { validatePlatformP2PPort, validatePlatformHTTPPort } = createPortValidators(
      defaultConfigs,
      ctx.preset,
    );

    const core = ctx.config.get('core.p2p.port');

    const form = await promptOrThrow(task, {
      type: 'form',
      header: `  Platform peers and clients reach this node on these ports, at its external
  IP address. Make sure they are open to the public internet.\n`,
      message: 'Enter the Platform ports:',
      choices: [
        {
          name: 'platformP2PPort',
          message: 'Platform P2P port',
          initial: String(ctx.config.get(P2P_PORT_PATH)),
          validate: validatePlatformP2PPort,
        },
        {
          name: 'platformHTTPPort',
          message: 'Platform HTTP port',
          initial: String(ctx.config.get(HTTP_PORT_PATH)),
          validate: validatePlatformHTTPPort,
        },
      ],
      validate: ({ platformP2PPort, platformHTTPPort }) => {
        const p2pValidity = validatePlatformP2PPort(platformP2PPort);
        const httpValidity = validatePlatformHTTPPort(platformHTTPPort);

        if (p2pValidity !== true || httpValidity !== true) {
          return p2pValidity === true ? httpValidity : p2pValidity;
        }

        return findPortCollision({
          core,
          p2p: Number(platformP2PPort),
          http: Number(platformHTTPPort),
        }) ?? true;
      },
    }, { interactive: ctx.interactive });

    return { p2p: Number(form.platformP2PPort), http: Number(form.platformHTTPPort) };
  }

  /**
   * @param {Object} ctx
   * @return {string}
   */
  function renderSummary(ctx) {
    const { config } = ctx;
    const flag = renderConfigFlag(config.getName());
    const ip = config.get('externalIp');

    const lines = [
      `Platform of ${config.getName()} is set up.`,
      '',
      `  Node type       evolution ${ctx.isMasternode ? 'masternode' : 'fullnode'}`,
      `  Node ID         ${config.get('platform.drive.tenderdash.node.id')}${ctx.isMasternode ? ' (matches the registration)' : ''}`,
      `  Platform P2P    ${ip}:${config.get(P2P_PORT_PATH)}`,
      `  Platform HTTPS  ${ip}:${config.get(HTTP_PORT_PATH)}`,
      `  Certificate     ${config.get('platform.gateway.ssl.provider')}`,
      '',
    ];

    if (!ctx.isMasternode && !ctx.wasPlatformEnabled) {
      lines.push('Platform was disabled on this fullnode; it is an evolution fullnode now.', '');
    }

    if (ctx.registration?.poSeBanHeight) {
      lines.push(
        `This masternode is PoSe-banned (since block ${ctx.registration.poSeBanHeight}). Once Platform`,
        'is running, revive it with a ProUpServTx with its current node ID and endpoints:',
        'DMT "Update service", or `protx update_service_evo` in the Dash-Qt console.',
        '',
      );
    }

    lines.push(
      'Next:',
      `  dashmate update ${flag}`,
      ctx.isCoreRunning
        ? `  dashmate start --platform ${flag}`
        : `  dashmate start ${flag}`,
    );

    if (ctx.isExternalIpWritten && ctx.isCoreRunning) {
      lines.push('Core advertises the new external IP the next time it starts.');
    }

    return lines.join('\n');
  }

  /**
   * Configure the Platform part of a preset whose Core part is set up
   *
   * Everything is held on the context's copy of the config until the last step
   * saves and renders it, which happens only if every step before succeeded.
   * The context carries the answers given in advance (see the setup command)
   * and states whether prompting is allowed.
   *
   * @typedef {function} setupPlatformTask
   * @return {Listr}
   */
  function setupPlatformTask() {
    return new Listr([
      {
        task: () => verifySystemRequirementsTask(),
      },
      {
        title: 'Connect to Core',
        enabled: (ctx) => ctx.isMasternode,
        task: async (ctx, task) => {
          const { config } = ctx;

          if (await dockerCompose.isServiceRunning(config, 'core')) {
            const rpcClient = createRpcClient({
              port: config.get('core.rpc.port'),
              user: 'dashmate',
              pass: config.get('core.rpc.users.dashmate.password'),
              host: await getConnectionHost(config, 'core', 'core.rpc.host'),
            });

            ctx.coreService = { getRpcClient: () => rpcClient };

            // eslint-disable-next-line no-param-reassign
            task.output = 'Core is running';

            return;
          }

          if (ctx.interactive) {
            const isStartAccepted = await promptOrThrow(task, {
              type: 'toggle',
              header: `  Core is not running. Setup needs it to read this masternode's registration
  from the network. It starts Core now and stops it again when it exits.\n`,
              message: 'Start Core?',
              enabled: 'Yes',
              disabled: 'No',
              initial: true,
            }, { interactive: ctx.interactive });

            if (!isStartAccepted) {
              throw new Error('Setting up Platform on a masternode needs Core, to read the'
                + ' masternode\'s registration');
            }
          }

          // Compose creates the volumes of every service its profiles enable,
          // `run core` included (2.24 does), and labels them as this project's.
          // A Platform volume left by a run that then fails would refuse the
          // next run, as if a reset had kept Platform data, so Core runs with
          // every Platform profile off.
          const coreOnlyConfig = new Config(config.getName(), config.getStoredOptions());
          coreOnlyConfig.set('platform.enable', false);
          coreOnlyConfig.set('platform.quorumList.enabled', false);

          ctx.coreService = await startCore(coreOnlyConfig);
          ctx.isCoreRunning = false;
          // The command stops it gracefully when the run ends, success or not.
          ctx.isCoreStartedBySetup = true;

          // eslint-disable-next-line no-param-reassign
          task.output = 'Core started; it stops again when setup exits';
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Wait for Core to sync',
        enabled: (ctx) => ctx.isMasternode,
        task: (ctx) => new Observable((observer) => {
          waitForCoreSync(ctx.coreService, ({ percent, headers, blocks }) => {
            observer.next(`${(percent * 100).toFixed(2)}% complete (${blocks} / ${headers})`);
          }).then(() => observer.complete(), (e) => observer.error(e));
        }),
      },
      {
        title: 'Find the registration of this operator key',
        enabled: (ctx) => ctx.isMasternode,
        task: async (ctx, task) => {
          const registration = await readPlatformRegistration(ctx.coreService.getRpcClient(), {
            operatorPrivateKey: ctx.config.get('core.masternode.operator.privateKey'),
            externalIp: ctx.config.get('externalIp'),
          });

          switch (registration.status) {
            case REGISTRATION_STATUS.EVO:
              ctx.registration = registration;

              // eslint-disable-next-line no-param-reassign
              task.output = `Evo masternode ${registration.proTxHash}, node ID`
                + ` ${registration.platformNodeId}`;

              return;
            case REGISTRATION_STATUS.REGULAR:
              throw new Error(`This masternode (${registration.proTxHash}) is registered as`
                + ' Regular. Platform needs an Evo registration');
            case REGISTRATION_STATUS.NOT_FOUND:
              throw new Error('No registered masternode uses this node\'s operator key. Check'
                + ' core.masternode.operator.privateKey, or wait until the ProRegTx is mined');
            case REGISTRATION_STATUS.UNSUPPORTED_ENDPOINTS:
              throw new Error(`${registration.reason}.\ndashmate serves Platform only on the`
                + ' external IP, with one P2P and one HTTPS port, so it cannot serve this'
                + ' registration. If this node\'s externalIp is the one out of date, set it with'
                + ` 'dashmate config set externalIp <ip> ${renderConfigFlag(ctx.config.getName())}'`);
            default:
              throw new Error(`Cannot read the registration: ${registration.reason}`);
          }
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Platform node key',
        task: async (ctx, task) => {
          const nodeKey = ctx.isMasternode
            ? await getRegisteredNodeKey(ctx, task)
            : await getFullnodeNodeKey(ctx, task);

          ctx.config.set('platform.drive.tenderdash.node.key', nodeKey.key);
          ctx.config.set('platform.drive.tenderdash.node.id', nodeKey.id);

          // eslint-disable-next-line no-param-reassign
          task.output = `node ID ${nodeKey.id}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Platform ports',
        task: async (ctx, task) => {
          let ports;

          if (ctx.isMasternode) {
            const { platformP2PPort: p2p, platformHttpPort: http } = ctx.registration;

            // Answers given in advance may repeat the registration, so a
            // script can run unchanged; they cannot change it.
            [
              ['--platform-p2p-port', ctx.platformP2PPort, p2p],
              ['--platform-http-port', ctx.platformHttpPort, http],
            ].forEach(([flag, answer, registered]) => {
              if (answer !== undefined && answer !== registered) {
                throw new Error(`${flag} ${answer} is not the registered port ${registered}. A`
                  + ' masternode serves Platform on the ports its registration records');
              }
            });

            ports = { p2p, http };
          } else {
            ports = await getFullnodeEndpoints(ctx, task);

            // Stored ports are taken as they are, and the schema lets 0 through.
            const { validatePlatformP2PPort, validatePlatformHTTPPort } = createPortValidators(
              defaultConfigs,
              ctx.preset,
            );
            [
              ['P2P', ports.p2p, validatePlatformP2PPort],
              ['HTTP', ports.http, validatePlatformHTTPPort],
            ].forEach(([label, port, validate]) => {
              if (validate(String(port)) !== true) {
                throw new Error(`The Platform ${label} port ${port} cannot be used. Give another with`
                  + ` --platform-${label.toLowerCase()}-port`);
              }
            });

            const collision = findPortCollision({ core: ctx.config.get('core.p2p.port'), ...ports });

            if (collision) {
              throw new Error(`Cannot use these Platform ports: ${collision}`);
            }
          }

          ctx.config.set(P2P_PORT_PATH, ports.p2p);
          ctx.config.set(HTTP_PORT_PATH, ports.http);

          // eslint-disable-next-line no-param-reassign
          task.output = `P2P ${ports.p2p}, HTTP ${ports.http}`
            + `${ctx.isMasternode ? ' (registered)' : ''}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'External IP address',
        // Every certificate is issued for it, self-signed included. A masternode
        // has one already: its registration was checked against it.
        enabled: (ctx) => !ctx.isMasternode && ctx.config.get('externalIp') === null,
        task: async (ctx, task) => {
          let ip = ctx.externalIpAnswer;

          if (ip === undefined) {
            const initial = await Promise.race([
              publicIpv4().catch(() => ''),
              // Give up on the lookup after 10 seconds
              wait(10000).then(() => ''),
            ]);

            ip = await promptOrThrow(task, {
              type: 'input',
              header: `  Platform peers and clients reach this node at its public IPv4 address,
  and the certificate is issued for it.\n`,
              message: 'External IP address',
              initial,
              validate: (value) => validateIPv4(value) || 'Enter an IPv4 address',
            }, { interactive: ctx.interactive });
          }

          ctx.config.set('externalIp', ip);
          ctx.isExternalIpWritten = true;

          // eslint-disable-next-line no-param-reassign
          task.output = ip;
        },
        options: { persistentOutput: true },
      },
      {
        task: (ctx) => {
          ctx.isCertificateStepStarted = true;

          return configureSSLCertificateTask();
        },
      },
      {
        title: 'Save the configuration',
        task: async (ctx, task) => {
          // `start` does not take the config lock, so Platform could have been
          // started while setup waited on the certificate. The config as it was
          // names the same services; the answers stay out of the environment
          // Compose runs with, where debug output of child processes prints it.
          const activeServices = await getPlatformArtefacts.getActiveServices(ctx.storedConfig);

          if (activeServices.length > 0) {
            throw new Error(`Platform services were started while setup ran: ${activeServices.join(', ')}.`
              + ` Stop them with 'dashmate stop --platform ${renderConfigFlag(ctx.config.getName())}'`
              + ' and run setup again');
          }

          configFile.setConfig(ctx.config);

          // Written here rather than when the command exits, so that a failure
          // is a failed setup, reported as one: before config.json is written,
          // the certificate files this run installed are removed so that it can
          // run again; after, config.json refers to them and they stay.
          configFileRepository.write(configFile);
          ctx.isConfigSaved = true;

          // Core's service files read no Platform option, so they are left as
          // they are unless setup changed another option (externalIp, which
          // dash.conf advertises). Rendered again, dash.conf would lose an
          // operator's edit and get new rpcauth salts.
          const isOnlyPlatformChanged = lodash.isEqual(
            lodash.omit(ctx.storedConfig.getOptions(), 'platform'),
            lodash.omit(ctx.config.getOptions(), 'platform'),
          );

          writeConfigTemplates(
            ctx.config,
            isOnlyPlatformChanged ? PLATFORM_ONLY_RENDER_SCOPE : undefined,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = renderSummary(ctx);
        },
        options: {
          persistentOutput: true,
          rendererOptions: {
            bottomBar: true,
          },
        },
      },
    ]);
  }

  return setupPlatformTask;
}

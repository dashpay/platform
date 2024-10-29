import fs from 'fs';
import { Listr } from 'listr2';
import path from 'path';
import process from 'process';
import si from 'systeminformation';
import obfuscateConfig from '../../../config/obfuscateConfig.js';
import { DASHMATE_VERSION } from '../../../constants.js';
import Certificate from '../../../ssl/zerossl/Certificate.js';
import providers from '../../../status/providers.js';
import hideString from '../../../util/hideString.js';
import obfuscateObjectRecursive from '../../../util/obfuscateObjectRecursive.js';
import validateSslCertificateFiles from '../../prompts/validators/validateSslCertificateFiles.js';

/**
 *
 * @param {string} url
 * @return {Promise<string>}
 */
async function fetchTextOrError(url) {
  try {
    const response = await fetch(url);

    return await response.text();
  } catch (e) {
    return e.toString();
  }
}

/**
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @param {createTenderdashRpcClient} createTenderdashRpcClient
 * @param {getServiceList} getServiceList
 * @param {getOperatingSystemInfo} getOperatingSystemInfo
 * @param {HomeDir} homeDir
 * @param {validateZeroSslCertificate} validateZeroSslCertificate
 * @return {collectSamplesTask}
 */
export default function collectSamplesTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
  createTenderdashRpcClient,
  getServiceList,
  getOperatingSystemInfo,
  homeDir,
  validateZeroSslCertificate,
) {
  /**
   * @typedef {function} collectSamplesTask
   * @param config
   * @return {Listr}
   */
  function collectSamplesTask(config) {
    return new Listr(
      [
        {
          title: 'System information',
          task: async (ctx) => {
            // Sample docker installation errors
            try {
              await dockerCompose.throwErrorIfNotInstalled();
            } catch (e) {
              ctx.samples.setDockerError(e);
            }

            // Operating system info
            const osInfo = await getOperatingSystemInfo();

            ctx.samples.setSystemInfo(osInfo);
          },
        },
        {
          title: 'Configuration',
          task: async (ctx) => {
            ctx.samples.setDashmateVersion(DASHMATE_VERSION);
            ctx.samples.setDashmateConfig(obfuscateConfig(config));

            return new Listr([
              {
                enabled: () => config.get('platform.enable'),
                title: 'Gateway SSL certificates',
                task: async () => {
                  if (!config.get('platform.gateway.ssl.enabled')) {
                    ctx.samples.setServiceInfo('gateway', 'ssl', {
                      error: 'disabled',
                    });

                    return;
                  }

                  switch (config.get('platform.gateway.ssl.provider')) {
                    case 'self-signed': {
                      ctx.samples.setServiceInfo('gateway', 'ssl', {
                        error: 'self-signed',
                      });

                      return;
                    }
                    case 'zerossl': {
                      const {
                        error,
                        data,
                      } = validateZeroSslCertificate(config, Certificate.EXPIRATION_LIMIT_DAYS);

                      obfuscateObjectRecursive(data, (_field, value) => (typeof value === 'string' ? value.replaceAll(
                        process.env.USER,
                        hideString(process.env.USER),
                      ) : value));

                      ctx.samples.setServiceInfo('gateway', 'ssl', {
                        error,
                        data,
                      });

                      return;
                    }
                    case 'file': {
                      // SSL certificate
                      const certificatesDir = homeDir.joinPath(
                        config.getName(),
                        'platform',
                        'gateway',
                        'ssl',
                      );

                      const chainFilePath = path.join(certificatesDir, 'bundle.crt');
                      const privateFilePath = path.join(certificatesDir, 'private.key');

                      const data = {
                        chainFilePath,
                        privateFilePath,
                      };

                      obfuscateObjectRecursive(data, (_field, value) => (typeof value === 'string' ? value.replaceAll(
                        process.env.USER,
                        hideString(process.env.USER),
                      ) : value));

                      if (!fs.existsSync(chainFilePath) || !fs.existsSync(privateFilePath)) {
                        ctx.samples.setServiceInfo('gateway', 'ssl', {
                          error: 'not-exist',
                          data,
                        });

                        return;
                      }

                      const isValid = validateSslCertificateFiles(chainFilePath, privateFilePath);

                      if (!isValid) {
                        ctx.samples.setServiceInfo('gateway', 'ssl', {
                          error: 'not-valid',
                          data,
                        });
                      }

                      return;
                    }
                    default:
                      throw new Error('Unknown SSL provider');
                  }
                },
              },
              {
                title: 'Core P2P port',
                task: async () => {
                  const port = config.get('core.p2p.port');
                  const response = await providers.mnowatch.checkPortStatus(port, config.get('externalIp'))
                    .catch((e) => e.toString());

                  ctx.samples.setServiceInfo('core', 'p2pPort', response);
                },
              },
              {
                title: 'Gateway HTTP port',
                enabled: () => config.get('platform.enable'),
                task: async () => {
                  const port = config.get('platform.gateway.listeners.dapiAndDrive.port');
                  const response = await providers.mnowatch.checkPortStatus(port, config.get('externalIp'))
                    .catch((e) => e.toString());

                  ctx.samples.setServiceInfo('gateway', 'httpPort', response);
                },
              },
              {
                title: 'Tenderdash P2P port',
                task: async () => {
                  const port = config.get('platform.drive.tenderdash.p2p.port');
                  const response = await providers.mnowatch.checkPortStatus(port, config.get('externalIp'))
                    .catch((e) => e.toString());

                  ctx.samples.setServiceInfo('drive_tenderdash', 'p2pPort', response);
                },
              },
            ]);
          },
        },
        {
          title: 'Core status',
          task: async (ctx) => {
            const rpcClient = createRpcClient({
              port: config.get('core.rpc.port'),
              user: 'dashmate',
              pass: config.get('core.rpc.users.dashmate.password'),
              host: await getConnectionHost(config, 'core', 'core.rpc.host'),
            });

            const coreCalls = [
              rpcClient.getBestChainLock(),
              rpcClient.quorum('listextended'),
              rpcClient.getBlockchainInfo(),
              rpcClient.getPeerInfo(),
              rpcClient.mnsync('status'),
            ];

            if (config.get('core.masternode.enable')) {
              coreCalls.push(rpcClient.masternode('status'));
            }

            const [
              getBestChainLock,
              quorums,
              getBlockchainInfo,
              getPeerInfo,
              masternodeStatus,
              masternodeSyncStatus,
            ] = (await Promise.allSettled(coreCalls))
              .map((e) => e.value?.result || e.reason);

            ctx.samples.setServiceInfo('core', 'bestChainLock', getBestChainLock);
            ctx.samples.setServiceInfo('core', 'quorums', quorums);
            ctx.samples.setServiceInfo('core', 'blockchainInfo', getBlockchainInfo);
            ctx.samples.setServiceInfo('core', 'peerInfo', getPeerInfo);
            ctx.samples.setServiceInfo('core', 'masternodeStatus', masternodeStatus);
            ctx.samples.setServiceInfo('core', 'masternodeSyncStatus', masternodeSyncStatus);
          },
        },
        {
          title: 'Tenderdash status',
          enabled: () => config.get('platform.enable'),
          task: async (ctx) => {
            const tenderdashRPCClient = createTenderdashRpcClient({
              host: config.get('platform.drive.tenderdash.rpc.host'),
              port: config.get('platform.drive.tenderdash.rpc.port'),
            });

            // Tenderdash requires to pass all params, so we use basic fetch
            async function fetchValidators() {
              const url = `http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/validators?request_quorum_info=true`;
              const response = await fetch(url, 'GET');
              return response.json();
            }

            const [
              status,
              genesis,
              peers,
              abciInfo,
              consensusState,
              validators,
            ] = await Promise.allSettled([
              tenderdashRPCClient.request('status', []),
              tenderdashRPCClient.request('genesis', []),
              tenderdashRPCClient.request('net_info', []),
              tenderdashRPCClient.request('abci_info', []),
              tenderdashRPCClient.request('dump_consensus_state', []),
              fetchValidators(),
            ]);

            ctx.samples.setServiceInfo('drive_tenderdash', 'status', status);
            ctx.samples.setServiceInfo('drive_tenderdash', 'validators', validators);
            ctx.samples.setServiceInfo('drive_tenderdash', 'genesis', genesis);
            ctx.samples.setServiceInfo('drive_tenderdash', 'peers', peers);
            ctx.samples.setServiceInfo('drive_tenderdash', 'abciInfo', abciInfo);
            ctx.samples.setServiceInfo('drive_tenderdash', 'consensusState', consensusState);
          },
        },
        {
          title: 'Metrics',
          enabled: () => config.get('platform.enable'),
          task: async (ctx, task) => {
            if (config.get('platform.drive.tenderdash.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Tenderdash metrics';

              const url = `http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/metrics`;

              const result = fetchTextOrError(url);

              ctx.samples.setServiceInfo('drive_tenderdash', 'metrics', result);
            }

            if (config.get('platform.drive.abci.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Drive metrics';

              const url = `http://${config.get('platform.drive.abci.rpc.host')}:${config.get('platform.drive.abci.rpc.port')}/metrics`;

              const result = fetchTextOrError(url);

              ctx.samples.setServiceInfo('drive_abci', 'metrics', result);
            }

            if (config.get('platform.gateway.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Gateway metrics';

              const url = `http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`;

              const result = fetchTextOrError(url);

              ctx.samples.setServiceInfo('gateway', 'metrics', result);
            }
          },
        },
        {
          title: 'Docker containers info',
          task: async (ctx) => {
            const services = await getServiceList(config);

            await Promise.all(
              services.map(async (service) => {
                const [inspect, logs] = (await Promise.allSettled([
                  dockerCompose.inspectService(config, service.name),
                  dockerCompose.logs(config, [service.name], { tail: 300000 }),
                ])).map((e) => e.value || e.reason);

                const containerId = inspect?.Id;
                let dockerStats;
                if (containerId) {
                  dockerStats = await si.dockerContainerStats(containerId);
                }

                if (logs?.out) {
                  // Hide username & external ip from logs
                  logs.out = logs.out.replaceAll(
                    process.env.USER,
                    hideString(process.env.USER),
                  );
                }

                if (logs?.err) {
                  logs.err = logs.err.replaceAll(
                    process.env.USER,
                    hideString(process.env.USER),
                  );
                }

                // Hide username & external ip from inspect
                obfuscateObjectRecursive(inspect, (_field, value) => (
                  typeof value === 'string'
                    ? value.replaceAll(
                      process.env.USER,
                      hideString(process.env.USER),
                    )
                    : value
                ));

                ctx.samples.setServiceInfo(service.name, 'stdOut', logs?.out);
                ctx.samples.setServiceInfo(service.name, 'stdErr', logs?.err);
                ctx.samples.setServiceInfo(service.name, 'dockerInspect', inspect);
                ctx.samples.setServiceInfo(service.name, 'dockerStats', dockerStats);
              }),
            );
          },
        },
      ],
    );
  }

  return collectSamplesTask;
}

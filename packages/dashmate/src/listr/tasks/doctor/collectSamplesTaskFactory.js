import fs from 'fs';
import os from 'os';
import { Listr } from 'listr2';
import path from 'path';
import process from 'process';
import si from 'systeminformation';
import obfuscateConfig from '../../../config/obfuscateConfig.js';
import { DASHMATE_VERSION } from '../../../constants.js';
import LegoCertificate from '../../../ssl/letsencrypt/LegoCertificate.js';
import Certificate from '../../../ssl/zerossl/Certificate.js';
import probeServedCertificate, { STATE as PROBE_STATE } from '../../../ssl/probeServedCertificate.js';
import readCertificateBundle from '../../../ssl/readCertificateBundle.js';
import readRenewalRecord from '../../../ssl/renewalRecord.js';
import providers from '../../../status/providers.js';
import maskOperatorIdentity from '../../../util/maskOperatorIdentity.js';
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
 * @param {validateLetsEncryptCertificate} validateLetsEncryptCertificate
 * @param {checkGatewayCertificate} checkGatewayCertificate
 * @return {collectSamplesTask}
 */
/**
 * Mask the name of whoever is running dashmate out of collected data.
 *
 * A report is the artefact an operator hands to whoever is helping them, and
 * the paths in it are absolute, so they carry a home directory. The paths stay
 * - they are what makes a problem actionable - and the name in them does not.
 *
 * The name is read from the operating system rather than the environment.
 * Doctor runs unattended often enough - from cron, from a service manager -
 * that USER cannot be relied on, and replacing an undefined needle silently
 * masks nothing at all. When no name can be determined there is nothing to
 * mask and the data is left alone rather than having "undefined" replaced in
 * it.
 *
 * @return {{username: string|null, homePath: string|null}}
 */
function getOperatorIdentity() {
  let username = null;
  let homePath = null;

  try {
    ({ username, homedir: homePath } = os.userInfo());
  } catch {
    // A process running under a uid with no passwd entry has no name to read.
  }

  return {
    username: username || process.env.USER || process.env.USERNAME || null,
    homePath: homePath || os.homedir() || null,
  };
}

/**
 * @param {Object} data - mutated in place
 */
function obfuscateOperatorName(data) {
  const identity = getOperatorIdentity();

  obfuscateObjectRecursive(data, (_field, value) => maskOperatorIdentity(value, identity));
}

/**
 * @param {string|undefined} text
 * @return {string|undefined}
 */
function hideOperatorNameIn(text) {
  return maskOperatorIdentity(text, getOperatorIdentity());
}

export default function collectSamplesTaskFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
  createTenderdashRpcClient,
  getServiceList,
  getOperatingSystemInfo,
  homeDir,
  validateZeroSslCertificate,
  validateLetsEncryptCertificate,
  checkGatewayCertificate,
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
                      } = await validateZeroSslCertificate(
                        config,
                        Certificate.EXPIRATION_LIMIT_DAYS,
                      );

                      obfuscateOperatorName(data);

                      ctx.samples.setServiceInfo('gateway', 'ssl', {
                        error,
                        data,
                      });

                      return;
                    }
                    case 'letsencrypt': {
                      const {
                        error,
                        data,
                      } = await validateLetsEncryptCertificate(
                        config,
                        LegoCertificate.EXPIRATION_LIMIT_DAYS,
                      );

                      obfuscateOperatorName(data);

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

                      obfuscateOperatorName(data);

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
                // Judged where the files are, because an archived report is
                // analysed somewhere else entirely. This is also the only
                // certificate sample a stopped node produces: the probe below
                // needs a listener to answer it, and the documented upgrade
                // procedure leaves the gateway down.
                enabled: () => config.get('platform.enable'),
                title: 'Gateway certificate files',
                task: async () => {
                  const verdict = checkGatewayCertificate(config);

                  const installed = {
                    status: verdict.status,
                    reasons: verdict.reasons,
                    warnings: verdict.warnings,
                    skipped: verdict.skipped,
                    provider: verdict.provider,
                    expiresInDays: verdict.expiresInDays,
                    validTo: verdict.installed
                      ? verdict.installed.validTo.toUTCString()
                      : null,
                    // When this certificate was issued, which is what says
                    // whether a recorded renewal failure came before it. A
                    // failure the certificate outlives has been overtaken.
                    validFrom: verdict.installed
                      ? verdict.installed.validFrom.toUTCString()
                      : null,
                    // Which pair was judged. The wire probe records the same
                    // fingerprint for the file it read, so an analyser can tell
                    // whether the two samples describe the same certificate
                    // before acting on the verdict.
                    fingerprint256: verdict.installed
                      ? verdict.installed.fingerprint256
                      : null,
                  };

                  // A problem with the files names the file it could not read,
                  // which is an absolute path under the operator's home
                  // directory. The report this ends up in is what an operator
                  // hands to whoever is helping them, so the path stays - it is
                  // what makes the problem actionable - and the name in it does
                  // not.
                  obfuscateOperatorName(installed);

                  ctx.samples.setServiceInfo('gateway', 'installedCertificate', installed);
                },
              },
              {
                // Read next to the certificate it describes rather than
                // anywhere else in this collection. The helper replaces the
                // certificate and writes this seconds apart, and the rest of
                // the collection takes long enough - there is a call out to the
                // internet in it - that reading them minutes apart would
                // routinely straddle a renewal and report a node that has just
                // succeeded as one that is failing.
                enabled: () => config.get('platform.enable'),
                title: 'Gateway certificate renewal',
                task: async () => {
                  const renewal = readRenewalRecord(homeDir, config.getName());

                  // Absent and unreadable are kept apart all the way to the
                  // analyser. "Nothing was recorded" is a fair thing to say;
                  // saying it about a file that could not be opened is not.
                  const sample = {
                    state: renewal.state,
                    path: renewal.path,
                    error: renewal.error,
                    ...renewal.record,
                  };

                  // Same treatment as the certificate above: the path is what
                  // makes a problem actionable and stays, the operator's name
                  // in it does not.
                  obfuscateOperatorName(sample);

                  ctx.samples.setServiceInfo('gateway', 'certificateRenewal', sample);
                },
              },
              {
                // Every other certificate check reads a file or the provider's API, so a
                // certificate that was renewed on disk but never reached the gateway looks
                // healthy to all of them. This connects to the gateway and records what it
                // actually serves. Doctor is run by an operator on the node, so the gateway's
                // listener is reached at the address it is published on.
                enabled: () => config.get('platform.enable')
                  && config.get('platform.gateway.ssl.provider') !== 'self-signed',
                title: 'Gateway served certificate',
                task: async () => {
                  const listenerHost = config.get('platform.gateway.listeners.dapiAndDrive.host');
                  const port = config.get('platform.gateway.listeners.dapiAndDrive.port');

                  const result = await probeServedCertificate({
                    host: listenerHost === '0.0.0.0' ? '127.0.0.1' : listenerHost,
                    port,
                    externalIp: config.get('externalIp'),
                  });

                  result.port = port;

                  if (result.state === PROBE_STATE.SERVED) {
                    // Read beside the probe rather than at analysis time: renewal replaces the
                    // file and signals the gateway moments apart, and the rest of the sample
                    // collection takes long enough that the two would routinely be read from
                    // either side of a renewal and reported as a mismatch.
                    const onDisk = readCertificateBundle(path.join(
                      homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl'),
                      'bundle.crt',
                    ));

                    result.onDisk = onDisk && {
                      fingerprint256: onDisk.fingerprint256,
                      validTo: onDisk.validTo.toUTCString(),
                    };

                    result.matchesOnDisk = onDisk
                      ? onDisk.fingerprint256 === result.certificate.fingerprint256
                      : null;
                  }

                  ctx.samples.setServiceInfo('gateway', 'servedCertificate', result);
                },
              },
              {
                // Both obtainable providers reach this node on port 80 to prove it controls
                // its address before issuing: Let's Encrypt over ACME, ZeroSSL over its own
                // verification server. A self-signed or operator-supplied certificate is
                // never validated, so the port means nothing for those.
                enabled: () => config.get('platform.enable')
                  && ['zerossl', 'letsencrypt'].includes(config.get('platform.gateway.ssl.provider')),
                title: 'Certificate validation port',
                task: async () => {
                  const response = await providers.mnowatch.checkPortStatus(80, config.get('externalIp'))
                    .catch((e) => e.toString());

                  ctx.samples.setServiceInfo('gateway', 'validationHttpPort', response);
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

              const result = await fetchTextOrError(url);

              ctx.samples.setServiceInfo('drive_tenderdash', 'metrics', result);
            }

            if (config.get('platform.drive.abci.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Drive metrics';

              const url = `http://${config.get('platform.drive.abci.metrics.host')}:${config.get('platform.drive.abci.metrics.port')}/metrics`;

              const result = await fetchTextOrError(url);

              ctx.samples.setServiceInfo('drive_abci', 'metrics', result);
            }

            if (config.get('platform.gateway.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Gateway metrics';

              const url = `http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`;

              const result = await fetchTextOrError(url);

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
                  logs.out = hideOperatorNameIn(logs.out);
                }

                if (logs?.err) {
                  logs.err = hideOperatorNameIn(logs.err);
                }

                // Hide username & external ip from inspect
                obfuscateOperatorName(inspect);

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

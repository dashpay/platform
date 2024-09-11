import chalk from 'chalk';
import { Listr } from 'listr2';
import validateSslCertificateFiles from "../../prompts/validators/validateSslCertificateFiles.js";
import fs from "fs";
import path from "path";
import validateZeroSslCertificateFactory, {ERRORS} from "../../../ssl/zerossl/validateZeroSslCertificateFactory.js";
import Certificate from "../../../ssl/zerossl/Certificate.js";

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {getServiceList} getServiceList
 * @param {verifySystemRequirements} verifySystemRequirements
 * @param {HomeDir} homeDir
 * @param {validateZeroSslCertificate} validateZeroSslCertificate
 * @return {analyseSamplesTask}
 */
export default function analyseSamplesTaskFactory(
  dockerCompose,
  getServiceList,
  verifySystemRequirements,
  homeDir,
  validateZeroSslCertificate,
) {
  /**
   * @typedef {function} analyseSamplesTask
   * @param config
   * @return {Listr}
   */
  function analyseSamplesTask(config) {
    return new Listr([
      {
        title: 'System resources',
        task: async (ctx) => {
          const {
            cpu,
            dockerSystemInfo,
            currentLoad,
            diskSpace,
            fsOpenFiles,
            memory,
            diskIO,
          } = ctx.samples.getSystemInfo();

          ctx.systemResourceProblems = verifySystemRequirements(
            {
              dockerSystemInfo,
              cpu,
              memory,
              diskSpace,
            },
            config.get('platform.enabled'),
            {
              diskSpace: 5,
            },
          );

          return new Listr([
            {
              title: 'CPU cores',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.cpuCores) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'CPU speed',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.cpuSpeed) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'CPU load',
              task: (_ctx, task) => {
                if (currentLoad.avgLoad > 0.8) {
                  ctx.systemResourceProblems.avgLoad = `Average system load ${currentLoad.avgLoad.toFixed(2)} is higher than normal. Consider to upgrade CPU.`;
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Total RAM',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.memory) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Free RAM',
              enabled: Number.isInteger(memory.free),
              task: (_ctx, task) => {
                if (memory.free) {
                  const memoryGb = memory.free / (1024 ** 3);
                  if (memoryGb < 0.5) {
                    ctx.systemResourceProblems.freeMemory = `Only ${memoryGb.toFixed(1)}GB RAM is available. Consider to upgrade RAM.`;
                    throw new Error(task.title);
                  }
                }
              },
            },
            {
              title: 'File descriptors',
              enabled: fsOpenFiles?.allocated && fsOpenFiles?.max,
              task: (_ctx, task) => {
                const available = fsOpenFiles.max - fsOpenFiles.allocated;
                if (available < 1000) {
                  ctx.systemResourceProblems.fsOpenFiles = `${available} available file descriptors left. Consider to increase max limit.`;
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Swap',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.swap) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Disk space',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.diskSpace) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Disk IO',
              enabled: diskIO?.tWaitPercent,
              task: (_ctx, task) => {
                const THRESHOLD = 40;

                const maxDiskIOWaitPercent = Math.max(
                  diskIO.rWaitPercent,
                  diskIO.wWaitPercent,
                  diskIO.tWaitPercent,
                ) * 100;

                if (maxDiskIOWaitPercent > THRESHOLD) {
                  ctx.systemResourceProblems.diskIO = `Disk IO wait time is ${maxDiskIOWaitPercent.toFixed(0)}%. Consider to upgrade to faster disk.`;
                  throw new Error(task.title);
                }
              },
            },
          ], {
            exitOnError: false,
          });
        },
      },
      {
        title: 'Docker is started',
        task: async (ctx, task) => {
          try {
            await dockerCompose.throwErrorIfNotInstalled();
          } catch (e) {
            ctx.problems.push(e.message);
            ctx.skipOthers = true;

            throw new Error(task.title);
          }
        },
      },
      {
        title: 'Services are started',
        skip: (ctx) => ctx.skipOthers,
        task: async (ctx) => {
          const services = getServiceList(config);

          ctx.servicesNotStarted = [];
          ctx.servicesFailed = [];
          ctx.servicesOOMKilled = [];

          return new Listr(
            services.map((service) => (
              {
                title: service.title,
                task: () => {
                  const dockerInspect = ctx.samples.getServiceInfo(service.name, 'dockerInspect');

                  if (dockerInspect.message) {
                    ctx.servicesNotStarted.push({
                      service,
                      message: dockerInspect.message,
                    });
                  } else if (
                    dockerInspect.State.Restarting === true
                    && dockerInspect.State.ExitCode !== 0
                  ) {
                    ctx.servicesFailed.push({
                      service,
                      message: dockerInspect.State.Error,
                      code: dockerInspect.State.ExitCode,
                    });
                  } else if (dockerInspect.State.OOMKilled === true) {
                    ctx.servicesOOMKilled.push({
                      service,
                    });
                  } else {
                    return;
                  }

                  throw new Error(service.title);
                },
              }
            )),
            {
              exitOnError: false,
            },
          );
        },
      },
      {
        skip: (ctx) => ctx.skipOthers,
        task: async (ctx) => {
          if (ctx.servicesNotStarted.length > 0) {
            let problem;
            if (ctx.servicesNotStarted.length === 1) {
              problem = chalk`Service ${ctx.servicesNotStarted[0].service.title} isn't started.`
            } else {
              problem = chalk`Services ${ctx.servicesNotStarted.map((e) => e.service.title).join(', ')} aren't started.`
            }

            problem += chalk`\n\nTry {bold.cyanBright dashmate start --force} to make sure all services are started`;

            ctx.problems.push(problem);
          }

          if (ctx.servicesFailed.length > 0) {
            let problem;
            if (ctx.servicesFailed.length === 1) {
              const failedService = ctx.servicesFailed[0];

              problem = chalk`Service ${failedService.service.title} failed with an error code ${failedService.code}`;

              if (failedService.message?.length > 0) {
                problem += `and message: ${failedService.message}`;
              }

              problem += '.';
            } else {
              problem = chalk`${ctx.servicesFailed.length} services failed:`;

              ctx.servicesFailed.map((failedService) => {
                let output = chalk`  ${failedService.service.title} failed with an error code ${failedService.code}`;

                if (failedService.message?.length > 0) {
                  output += `and message: ${failedService.message}`;
                }

                output += '.';

                return output;
              }).join('\n');
            }

            problem += chalk`\n\nPlease check corresponding logs or share them with Dash Core Group`;

            ctx.problems.push(problem);
          }

          if (ctx.servicesOOMKilled.length > 0) {
            let problem;
            if (ctx.servicesNotStarted.length === 1) {
              problem = chalk`Service ${ctx.servicesNotStarted[0].service.title} is killed due to lack of memory.`;
            } else {
              problem = chalk`Services ${ctx.servicesNotStarted.map((e) => e.service.title).join(', ')} aren't killed due to lack of memory.`;
            }

            problem += chalk`\n\nMake sure you have enough memory to run the node.`;

            ctx.problems.push(problem);
          }

          const systemResourceProblems = Object.values(ctx.systemResourceProblems);
          if (systemResourceProblems.length > 0) {
            let problem;
            if (systemResourceProblems.length === 1) {
              problem = chalk`A system resource warning:\n${systemResourceProblems[0]}`;
            } else {
              problem = chalk`${systemResourceProblems.length} system resource warnings:\n${systemResourceProblems.join('\n')}`;
            }

            ctx.problems.push(problem);
          }
        },
      },
      {
        title: 'Configuration',
        task: async () => (
          new Listr([
            {
              title: 'Gateway admin is enabled if metrics are enabled',
              enabled: config.get('platform.enabled'),
              task: async (ctx, task) => {
                if (config.get('platform.gateway.metrics.enabled') && !config.get('platform.gateway.admin.enabled')) {
                  const problem = chalk`Gateway admin is disabled while metrics are enabled

Please enable gateway admin: {bold.cyanBright dashmate config set platform.gateway.admin.enabled true}`;

                  ctx.problems.push(problem);

                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Platform Node ID',
              enabled: config.get('platform.enabled'),
              task: async (ctx, task) => {
                const masternodeStatus = ctx.samples.getServiceInfo('core', 'masternodeStatus');
                const platformNodeId = masternodeStatus?.dmnState?.platformNodeId;
                if (platformNodeId && config.get('platform.drive.tenderdash.node.id') !== platformNodeId) {
                  const problem = chalk`Platform Node ID doesn't match the one in the ProReg transaction

Please set correct Node ID and Node Key:
{bold.cyanBright dashmate config set platform.drive.tenderdash.node.id ID
dashmate config set platform.drive.tenderdash.node.key KEY}

Or update Node ID in masternode list with ProServUp transaction`;

                  ctx.problems.push(problem);

                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'SSL certificate',
              enabled: config.get('platform.enabled'),
              task: async (ctx, task) => {
                const certificatesDir = homeDir.joinPath(
                  config.getName(),
                  'platform',
                  'gateway',
                  'ssl',
                );

                const chainFilePath = path.join(certificatesDir, 'bundle.crt');
                const privateFilePath = path.join(certificatesDir, 'private.key');

                if (!fs.existsSync(chainFilePath) || !fs.existsSync(privateFilePath)) {
                  const problem = chalk`SSL certificate files are not found

Certificate chain file path: {bold.cyanBright ${chainFilePath}}
Private key file path: {bold.cyanBright ${privateFilePath}}

Please get certificates and place files to the correct location.
Another optionUse ZeroSSL to obtain a new one https://docs.dash.org/en/stable/masternodes/dashmate.html#ssl-certificate`;

                  ctx.problems.push(problem);

                  throw new Error(task.title);
                }

                const isValid = validateSslCertificateFiles(chainFilePath, privateFilePath);

                if (!isValid) {
                  const problem = chalk`SSL certificate files aren't valid

Certificate chain file path: {bold.cyanBright ${chainFilePath}}
Private key file path: {bold.cyanBright ${privateFilePath}}

Please make sure certificate chain contains actual server certificate at the top of the file and it corresponds to private`;

                  ctx.problems.push(problem);

                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'ZeroSSL certificate',
              enabled: config.get('platform.gateway.ssl.provider') === 'zerossl',
              task: async (ctx, task) => {
                const {
                  error,
                  data,
                } = validateZeroSslCertificate(config, Certificate.EXPIRATION_LIMIT_DAYS);

                const problem = {
                  [ERRORS.API_KEY_IS_NOT_SET]: chalk`ZeroSSL API key is not set.

Please obtain your API key in {underline.cyanBright https://app.zerossl.com/developer}
And then update configuration with {block.cyanBright dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey [KEY]}`,
                  [ERRORS.EXTERNAL_IP_IS_NOT_SET]: chalk`External IP is not set.

Please update configuration with your external IP using {block.cyanBright dashmate config set externalIp [IP]}`,
                  [ERRORS.CERTIFICATE_ID_IS_NOT_SET]: chalk`ZeroSSL certificate is not configured

Please run {bold.cyanBright dashmate ssl obtain} to get a new one`,
                  [ERRORS.PRIVATE_KEY_IS_NOT_PRESENT]: chalk`ZeroSSL private key file not found in ${data.privateKeyFilePath}.

Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
                  [ERRORS.EXTERNAL_IP_MISMATCH]: chalk`ZeroSSL IP ${data.certificate.common_name} does not match external IP ${data.externalIp}.

Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
                  [ERRORS.CSR_FILE_IS_NOT_PRESENT]: chalk`ZeroSSL certificate request file not found in ${data.csrFilePath}.
This makes auto renew impossible.

If you need auto renew, please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
                  [ERRORS.CERTIFICATE_EXPIRES_SOON]: chalk`ZeroSSL certificate expires at ${data.certificate.expires}.

Please run {bold.cyanBright dashmate ssl obtain} to get a new one`,
                  [ERRORS.CERTIFICATE_IS_NOT_VALIDATED]: chalk`ZeroSSL certificate expires at ${data.certificate.expires}.

Please run {bold.cyanBright dashmate ssl obtain} to get a new one`,
                  [ERRORS.CERTIFICATE_IS_NOT_VALID]: chalk`ZeroSSL certificate is not valid.

  Please run {bold.cyanBright dashmate ssl zerossl obtain} to get a new one.`,
                }[error];

                if (problem) {
                  ctx.problems.push(problem);

                  throw new Error(task.title);
                }
              },
            },
            // TODO: Get checks from the status command
            // TODO: Errors in logs
          ], {
            exitOnError: false,
          })
        ),
      },
    ], {
      exitOnError: false,
    });
  }

  return analyseSamplesTask;
}

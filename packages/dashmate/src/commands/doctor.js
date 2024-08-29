import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import chalk from 'chalk';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import Report from '../doctor/report.js';
import { DASHMATE_VERSION } from '../constants.js';
import obfuscateConfig from '../config/obfuscateConfig.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';
import hideString from '../util/hideString.js';
import obfuscateObjectRecursive from '../util/obfuscateObjectRecursive.js';

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

export default class DoctorCommand extends ConfigBaseCommand {
  static description = 'Dashmate node diagnostic.  Bring your node to a doctor';

  static flags = {
    ...ConfigBaseCommand.flags,
    verbose: Flags.boolean({ char: 'v', description: 'use verbose mode for output', default: false }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param createRpcClient
   * @param {DockerCompose} dockerCompose
   * @param {getConnectionHost} getConnectionHost
   * @param {Config} config
   * @param createTenderdashRpcClient
   * @param getServiceList
   * @param getOperatingSystemInfo
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    { verbose: isVerbose },
    createRpcClient,
    dockerCompose,
    getConnectionHost,
    config,
    createTenderdashRpcClient,
    getServiceList,
    getOperatingSystemInfo,
  ) {
    const tasks = new Listr(
      [
        {
          task: async (ctx, task) => {
            const agreement = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk`  Dashmate is going to collect all necessary debug data from the node to create a report, including:

  - System information
  - The node configuration
  - Service logs, metrics and status

  Collected data will contain only anonymous information. All sensitive data like private keys or passwords is obfuscated.

  The report will be created as an TAR archive in {bold.cyanBright ${process.cwd()}}
  You can use the report to analyze your node condition yourself or send it to the Dash Core Group ({underline.cyanBright support@dash.org}) in case you need help.\n`,
              message: 'Create a report?',
              enabled: 'Yes',
              disabled: 'No',
            });

            if (!agreement) {
              throw new Error('Operation is cancelled');
            }

            ctx.report = new Report();
          },
        },
        {
          title: 'System information',
          task: async (ctx) => {
            const osInfo = await getOperatingSystemInfo();

            ctx.report.setSystemInfo(osInfo);
          },
        },
        {
          title: 'The node configuration',
          task: async (ctx) => {
            ctx.report.setDashmateVersion(DASHMATE_VERSION);
            ctx.report.setDashmateConfig(obfuscateConfig(config));
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
            ] = (await Promise.allSettled(coreCalls)).map((e) => e.value?.result || e.reason);

            ctx.report.setServiceInfo('core', 'bestChainLock', getBestChainLock);
            ctx.report.setServiceInfo('core', 'quorums', quorums);
            ctx.report.setServiceInfo('core', 'blockchainInfo', getBlockchainInfo);
            ctx.report.setServiceInfo('core', 'peerInfo', getPeerInfo);
            ctx.report.setServiceInfo('core', 'masternodeStatus', masternodeStatus);
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

            ctx.report.setServiceInfo('drive_tenderdash', 'status', status);
            ctx.report.setServiceInfo('drive_tenderdash', 'validators', validators);
            ctx.report.setServiceInfo('drive_tenderdash', 'genesis', genesis);
            ctx.report.setServiceInfo('drive_tenderdash', 'peers', peers);
            ctx.report.setServiceInfo('drive_tenderdash', 'abciInfo', abciInfo);
            ctx.report.setServiceInfo('drive_tenderdash', 'consensusState', consensusState);
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

              ctx.report.setServiceInfo('drive_tenderdash', 'metrics', result);
            }

            if (config.get('platform.drive.abci.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Drive metrics';

              const url = `http://${config.get('platform.drive.abci.rpc.host')}:${config.get('platform.drive.abci.rpc.port')}/metrics`;

              const result = fetchTextOrError(url);

              ctx.report.setServiceInfo('drive_abci', 'metrics', result);
            }

            if (config.get('platform.gateway.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.output = 'Reading Gateway metrics';

              const url = `http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`;

              const result = fetchTextOrError(url);

              ctx.report.setServiceInfo('gateway', 'metrics', result);
            }
          },
        },
        {
          title: 'Logs',
          task: async (ctx, task) => {
            const services = await getServiceList(config);

            // eslint-disable-next-line no-param-reassign
            task.output = `Pulling logs from ${services.map((e) => e.name)}`;

            await Promise.all(
              services.map(async (service) => {
                const [inspect, logs] = (await Promise.allSettled([
                  dockerCompose.inspectService(config, service.name),
                  dockerCompose.logs(config, [service.name]),
                ])).map((e) => e.value || e.reason);

                // Hide username & external ip from logs
                logs.out = logs.out.replaceAll(process.env.USER, hideString(process.env.USER));
                logs.err = logs.err.replaceAll(process.env.USER, hideString(process.env.USER));

                // Hide username & external ip from inspect
                obfuscateObjectRecursive(inspect, (_field, value) => (typeof value === 'string'
                  ? value.replaceAll(process.env.USER, hideString(process.env.USER)) : value));

                ctx.report.setServiceInfo(service.name, 'stdOut', logs.out);
                ctx.report.setServiceInfo(service.name, 'stdErr', logs.err);
                ctx.report.setServiceInfo(service.name, 'dockerInspect', inspect);
              }),
            );
          },
        },
        {
          title: 'Create an archive',
          task: async (ctx, task) => {
            const archivePath = process.cwd();

            await ctx.report.archive(archivePath);

            // eslint-disable-next-line no-param-reassign
            task.output = chalk`Saved to {bold.cyanBright ${archivePath}/dashmate-report-${ctx.report.date.toISOString()}.tar.gz}`;
          },
          options: {
            persistentOutput: true,
          },
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          showTimer: isVerbose,
          bottomBar: true,
          removeEmptyLines: false,
        },
      },
    );

    try {
      await tasks.run({
        isVerbose,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

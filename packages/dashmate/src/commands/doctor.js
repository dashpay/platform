import process from 'process';
import { Flags } from '@oclif/core';
import { Listr } from 'listr2';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import fetchHTTP from '../util/fetchHTTP.js';
import Report from '../doctor/report.js';
import { DASHMATE_VERSION } from '../constants.js';
import sanitizeDashmateConfig from '../util/sanitizeDashmateConfig.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';

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
              header: `Dashmate is going to collect all necessary debug data from the node, including:

* OS System Info (cpu, arch, disk, memory, inet)
* Docker Inspect & Logs for each dashmate service
* Dashmate Config (external ip, all keys and passwords stripped)
* Core RPC data (getbestchainlock, getblockchaininfo, quorums, getpeerinfo, masternode('status'))
* Tenderdash RPC data (if platform is enabled)
* Prometheus Metrics (if platform is enabled)

It will archived all collected info in an archive .tar archive in your current working directory (${process.cwd()})
You can use it to analyze your node condition yourself or send it to the Dash team in case you need help

support@dash.org
              `,
              message: 'Continue?',
              enabled: 'Yes',
              disabled: 'Abort',
            });

            if (!agreement) {
              throw new Error('Operation is cancelled');
            }

            ctx.report = new Report();
          },
        },
        {
          title: 'Gathering Operating System Info',
          task: async (ctx) => {
            const osInfo = await getOperatingSystemInfo();

            ctx.report.setOSInfo(osInfo);
          },
        },
        {
          title: 'Sanitizing Dashmate Config data',
          task: async (ctx) => {
            ctx.report.setDashmateVersion(DASHMATE_VERSION);
            ctx.report.setDashmateConfig(sanitizeDashmateConfig(config));
          },
        },
        {
          title: 'Requesting Core RPC node data',
          task: async (ctx) => {
            const rpcClient = createRpcClient({
              port: config.get('core.rpc.port'),
              user: 'dashmate',
              pass: config.get('core.rpc.users.dashmate.password'),
              host: await getConnectionHost(config, 'core', 'core.rpc.host'),
            });

            const coreCalls = [
              rpcClient.getBestChainLock(),
              rpcClient.quorum('list'),
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
          title: 'Collecting Tenderdash RPC status and consensus info',
          enabled: () => config.get('platform.enable'),
          task: async (ctx) => {
            const tenderdashRPCClient = createTenderdashRpcClient({
              host: config.get('platform.drive.tenderdash.rpc.host'),
              port: config.get('platform.drive.tenderdash.rpc.port'),
            });

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
              fetchHTTP(`http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/validators?request_quorum_info=true`, 'GET'),
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
          title: 'Reading Prometheus metrics',
          enabled: () => config.get('platform.enable'),
          task: async (ctx, task) => {
            if (config.get('platform.drive.tenderdash.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Reading Tenderdash metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);

              ctx.report.setServiceInfo('drive_tenderdash', 'metrics', metrics);
            }

            if (config.get('platform.drive.abci.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Reading Drive metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.drive.abci.rpc.host')}:${config.get('platform.drive.abci.rpc.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);

              ctx.report.setServiceInfo('drive_abci', 'metrics', metrics);
            }

            if (config.get('platform.gateway.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Reading Gateway metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);
              ctx.report.setServiceInfo('gateway', 'metrics', metrics);
            }
          },
        },
        {
          title: 'Pulling Docker Container Info & Logs',
          task: async (ctx, task) => {
            const services = await getServiceList(config);

            // eslint-disable-next-line no-param-reassign
            task.title = `Pulling logs from ${services.map((e) => e.name)}`;

            await Promise.all(
              services.map(async (service) => {
                const [inspect, logs] = (await Promise.allSettled([
                  dockerCompose.inspectService(config, service.name),
                  dockerCompose.logs(config, [service.name]),
                ])).map((e) => e.value || e.reason);

                ctx.report.setServiceInfo(service.name, 'stdOut', logs.out);
                ctx.report.setServiceInfo(service.name, 'stdErr', logs.err);
                ctx.report.setServiceInfo(service.name, 'dockerInspect', inspect);
              }),
            );
          },
        },
        {
          title: 'Archiving',
          task: async (ctx, task) => {
            const archivePath = process.cwd();

            await ctx.report.archive(archivePath);

            // eslint-disable-next-line no-param-reassign
            task.title = `Archive with all debug data created in the current working dir (${archivePath}/dashmate-report-${ctx.report.date.toISOString()}.tar)`;
          },
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
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

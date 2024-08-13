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
          title: 'Prepare',
          task: async (ctx) => {
            ctx.report = new Report();
          },
        },
        {
          title: 'Collecting Operating System Info',
          task: async (ctx) => {
            const osInfo = await getOperatingSystemInfo();

            ctx.report.setOSInfo(osInfo);
          },
        },
        {
          title: 'Collecting Dashmate Config data',
          task: async (ctx) => {
            ctx.report.setDashmateVersion(DASHMATE_VERSION);
            ctx.report.setDashmateConfig(sanitizeDashmateConfig(config));
          },
        },
        {
          title: 'Collecting Core data',
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
          title: 'Collecting Tenderdash info',
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
          title: 'Collecting metrics',
          enabled: () => config.get('platform.enable'),
          task: async (ctx, task) => {
            if (config.get('platform.drive.tenderdash.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Collecting Tenderdash metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);

              ctx.report.setServiceInfo('drive_tenderdash', 'metrics', metrics);
            }

            if (config.get('platform.drive.abci.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Collecting Drive metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.drive.abci.rpc.host')}:${config.get('platform.drive.abci.rpc.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);

              ctx.report.setServiceInfo('drive_abci', 'metrics', metrics);
            }

            if (config.get('platform.gateway.metrics.enabled')) {
              // eslint-disable-next-line no-param-reassign
              task.title = 'Collecting Gateway metrics';

              const metrics = (await Promise.allSettled([
                fetchHTTP(`http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`, 'GET')]))
                .map((e) => e.value || e.reason);
              ctx.report.setServiceInfo('gateway', 'metrics', metrics);
            }
          },
        },
        {
          title: 'Collecting Docker info & Container Logs',
          task: async (ctx, task) => {
            const services = await getServiceList(config);

            // eslint-disable-next-line no-param-reassign
            task.title = `Collecting logs from ${services.map((e) => e.name)}`;

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
          title: 'Archive',
          task: async (ctx, task) => {
            const archivePath = process.cwd();

            await ctx.report.archive(archivePath);

            // eslint-disable-next-line no-param-reassign
            task.title = `Archive with all logs created in the current working dir (${archivePath}/dashmate-report-${ctx.report.date.toISOString()}.tar)`;
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

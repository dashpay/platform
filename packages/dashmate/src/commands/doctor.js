import { Flags } from '@oclif/core';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import getMetrics from '../util/getMetrics.js';
import Report from '../doctor/report.js';
import getDockerContainerInfo from '../util/getDockerContainerInfo.js';
import process from 'process';

export default class RestartCommand extends ConfigBaseCommand {
  static description = 'Restart node';

  static flags = {
    ...ConfigBaseCommand.flags,
    platform: Flags.boolean({ char: 'p', description: 'restart only platform', default: false }),
    safe: Flags.boolean({ char: 's', description: 'wait for dkg before stop', default: false }),
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
    flags,
    createRpcClient,
    dockerCompose,
    getConnectionHost,
    config,
    createTenderdashRpcClient,
    getServiceList,
    getOperatingSystemInfo,
  ) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: 'dashmate',
      pass: config.get('core.rpc.users.dashmate.password'),
      host: await getConnectionHost(config, 'core', 'core.rpc.host'),
    });

    const tenderdashRPCClient = createTenderdashRpcClient({
      host: config.get('platform.drive.tenderdash.rpc.host'),
      port: config.get('platform.drive.tenderdash.rpc.port'),
    });

    const report = new Report();

    // OS INFO
    const osInfo = await getOperatingSystemInfo();
    report.setOSInfo(osInfo);

    // CORE PART ----

    // Docker Info
    const coreDockerInfo = await getDockerContainerInfo(config, 'core', dockerCompose);
    report.setData('core', 'dockerInfo', coreDockerInfo);

    // CORE: bestchainlock, quorums, blockchaininfo, peers, masternode status
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

    report.setData('core', 'bestChainLock', getBestChainLock);
    report.setData('core', 'quorums', quorums);
    report.setData('core', 'blockchainInfo', getBlockchainInfo);
    report.setData('core', 'peerInfo', getPeerInfo);
    report.setData('core', 'masternodeStatus', masternodeStatus);

    // TENDERDASH: status, genesis, peers, metrics, abci_info, dump_consensus_state
    const [
      status,
      genesis,
      peers,
      abciInfo,
      consensusState,
    ] = await Promise.allSettled([
      tenderdashRPCClient.request('status', []),
      tenderdashRPCClient.request('genesis', []),
      tenderdashRPCClient.request('net_info', []),
      tenderdashRPCClient.request('abci_info', []),
      tenderdashRPCClient.request('dump_consensus_state', []),
    ]);

    report.setData('drive_tenderdash', 'status', status);
    report.setData('drive_tenderdash', 'genesis', genesis);
    report.setData('drive_tenderdash', 'peers', peers);
    report.setData('drive_tenderdash', 'abciInfo', abciInfo);
    report.setData('drive_tenderdash', 'consensusState', consensusState);

    if (config.get('platform.drive.tenderdash.metrics.enabled')) {
      const tenderdashMetrics = await getMetrics(config.get('platform.drive.tenderdash.metrics.host'), config.get('platform.drive.tenderdash.metrics.port'));

      report.setData('drive_tenderdash', 'metrics', tenderdashMetrics);
    }

    if (config.get('platform.drive.abci.metrics.enabled')) {
      const driveMetrics = await getMetrics(config.get('platform.drive.abci.metrics.host'), config.get('platform.drive.abci.metrics.port'));

      report.setData('drive_abci', 'metrics', driveMetrics);
    }

    const services = await getServiceList(config);

    for (const service of services) {
      const info = await dockerCompose.inspectService(config, service.name);

      const { exitCode, err: stdErr, out: stdOut } = await dockerCompose.logs(
        config,
        [service.name],
      );

      const dockerInfo = {
        exitCode, status: info.State.Status, stdOut, stdErr,
      };

      report.setData(service.name, 'dockerInfo', dockerInfo);
    }

    const archivePath = process.cwd();

    await report.archive(archivePath);

    console.log(`Archive with all logs created in the current working dir (${archivePath}/dashmate-report-${report.id}.tar)`)
  }
}

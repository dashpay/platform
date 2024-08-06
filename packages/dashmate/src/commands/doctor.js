import process from 'process';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import getMetrics from '../util/getMetrics.js';
import Report from '../doctor/report.js';

export default class DoctorCommand extends ConfigBaseCommand {
  static description = 'Dashmate node diagnostic.  Bring your node to a doctor';

  static flags = {
    ...ConfigBaseCommand.flags,
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
    console.log('Collecting Operating System Info');
    const osInfo = await getOperatingSystemInfo();
    report.setOSInfo(osInfo);

    // CORE: bestchainlock, quorums, blockchaininfo, peers, masternode status
    console.log('Collecting Core data');
    const coreCalls = [
      rpcClient.getBestChainLock(),
      rpcClient.quorum('list'),
      rpcClient.getBlockchainInfo(),
      rpcClient.getPeerInfo(),
      rpcClient.masternode('status'),
    ];

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

    console.log('Collecting Tenderdash data');
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

    console.log('Collecting Drive & Tenderdash metrics');
    const [tenderdashMetrics, driveMetrics] = (await Promise.allSettled(
      [
        getMetrics(
          config.get('platform.drive.tenderdash.metrics.host'),
          config.get('platform.drive.tenderdash.metrics.port'),
        ),
        getMetrics(
          config.get('platform.drive.abci.metrics.host'),
          config.get('platform.drive.abci.metrics.port'),
        ),
      ],
    )).map((e) => e.value || e.reason);

    report.setData('drive_tenderdash', 'metrics', tenderdashMetrics);
    report.setData('drive_abci', 'metrics', driveMetrics);

    const services = await getServiceList(config);

    console.log(`Collecting logs from ${services.map((e) => e.name)}`);

    await Promise.all(
      services.map(async (service) => {
        const [inspect, logs] = await Promise.all([
          dockerCompose.inspectService(config, service.name),
          dockerCompose.logs(config, [service.name]),
        ]);

        const dockerInfo = {
          image: service.image,
          name: service.name,
          title: service.title,
          status: inspect.State.Status,
          exitCode: logs.exitCode,
          stdOut: logs.out,
          stdErr: logs.err,
        };

        report.setData(service.name, 'dockerInfo', dockerInfo);
      }),
    );

    const archivePath = process.cwd();

    await report.archive(archivePath);

    console.log(`Archive with all logs created in the current working dir (${archivePath}/dashmate-report-${report.id}.tar)`);
  }
}

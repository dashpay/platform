import process from 'process';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import fetchHTTP from '../util/fetchHTTP.js';
import Report from '../doctor/report.js';
import { DASHMATE_VERSION } from '../constants.js';
import sanitizeDashmateConfig from '../util/sanitizeDashmateConfig.js'

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
    report.setDashmateVersion(DASHMATE_VERSION);
    report.setDashmateConfig(sanitizeDashmateConfig(config));

    console.log('Collecting Core data');
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

    report.setServiceInfo('core', 'bestChainLock', getBestChainLock);
    report.setServiceInfo('core', 'quorums', quorums);
    report.setServiceInfo('core', 'blockchainInfo', getBlockchainInfo);
    report.setServiceInfo('core', 'peerInfo', getPeerInfo);
    report.setServiceInfo('core', 'masternodeStatus', masternodeStatus);

    if (config.get('platform.enable')) {
      console.log('Collecting Tenderdash data');
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

      report.setServiceInfo('drive_tenderdash', 'status', status);
      report.setServiceInfo('drive_tenderdash', 'validators', validators);
      report.setServiceInfo('drive_tenderdash', 'genesis', genesis);
      report.setServiceInfo('drive_tenderdash', 'peers', peers);
      report.setServiceInfo('drive_tenderdash', 'abciInfo', abciInfo);
      report.setServiceInfo('drive_tenderdash', 'consensusState', consensusState);

      if (config.get('platform.drive.tenderdash.metrics.enabled')) {
        console.log('Collecting Tenderdash metrics');

        const metrics = (await Promise.allSettled([
          fetchHTTP(`http://${config.get('platform.drive.tenderdash.rpc.host')}:${config.get('platform.drive.tenderdash.rpc.port')}/metrics`, 'GET')]))
          .map((e) => e.value || e.reason);

        report.setServiceInfo('drive_tenderdash', 'metrics', metrics);
      }

      if (config.get('platform.drive.abci.metrics.enabled')) {
        console.log('Collecting Drive metrics');

        const metrics = (await Promise.allSettled([
          fetchHTTP(`http://${config.get('platform.drive.abci.rpc.host')}:${config.get('platform.drive.abci.rpc.port')}/metrics`, 'GET')]))
          .map((e) => e.value || e.reason);

        report.setServiceInfo('drive_abci', 'metrics', metrics);
      }

      if (config.get('platform.gateway.metrics.enabled')) {
        console.log('Collecting Gateway metrics');

        const metrics = (await Promise.allSettled([
          fetchHTTP(`http://${config.get('platform.gateway.metrics.host')}:${config.get('platform.gateway.metrics.port')}/metrics`, 'GET')]))
          .map((e) => e.value || e.reason);
        report.setServiceInfo('gateway', 'metrics', metrics);
      }
    }

    const services = await getServiceList(config);

    console.log(`Collecting logs from ${services.map((e) => e.name)}`);

    await Promise.all(
      services.map(async (service) => {
        const [inspect, logs] = (await Promise.allSettled([
          dockerCompose.inspectService(config, service.name),
          dockerCompose.logs(config, [service.name]),
        ])).map((e) => e.value || e.reason);

        report.setServiceInfo(service.name, 'stdOut', logs.out);
        report.setServiceInfo(service.name, 'stdErr', logs.err);
        report.setServiceInfo(service.name, 'dockerInspect', inspect);
      }),
    );

    const archivePath = process.cwd();

    await report.archive(archivePath);

    console.log(`Archive with all logs created in the current working dir (${archivePath}/dashmate-report-${report.date.toISOString()}.tar)`);
  }
}

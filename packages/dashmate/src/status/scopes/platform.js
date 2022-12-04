const fetch = require('node-fetch');
const determineStatus = require('../determineStatus');
const ServiceStatusEnum = require('../../enums/serviceStatus');
const providers = require('../providers');

module.exports = async (coreService, dockerCompose, config) => {
  const {
    result: {
      IsSynced: coreIsSynced,
    },
  } = await coreService.getRpcClient().mnsync('status');

  let status = await determineStatus(dockerCompose, config, 'drive_tenderdash');

  const httpService = `${config.get('externalIp')}:${config.get('platform.dapi.envoy.http.port')}`;
  const httpPort = config.get('platform.dapi.envoy.http.port');
  const httpPortState = await providers.mnowatch.checkPortStatus(httpPort);

  const gRPCService = `${config.get('externalIp')}:${config.get('platform.dapi.envoy.grpc.port')}`;
  const gRPCPort = await providers.mnowatch.checkPortStatus(config.get('platform.dapi.envoy.grpc.port'));
  const gRPCPortState = await providers.mnowatch.checkPortStatus(gRPCPort);

  const p2pService = `${config.get('externalIp')}:${config.get('platform.drive.tenderdash.p2p.port')}`;
  const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
  const p2pPortState = await providers.mnowatch.checkPortStatus(p2pPort);

  const rpcService = `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`;
  const tenderdash = {
    version: null,
    catchingUp: null,
    lastBlockHeight: null,
    latestAppHash: null,
    peers: null,
    network: null,
  };

  if (coreIsSynced && status !== ServiceStatusEnum.not_started && status !== ServiceStatusEnum.restarting) {
    // Collecting platform data fails if Tenderdash is waiting for core to sync
    try {
      const tenderdashStatus = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`);

      const { node_info, sync_info } = await tenderdashStatus.json();
      const { version, network } = node_info;
      const { catching_up, latest_block_height, latest_app_hash } = sync_info;

      const tenderdashNetInfoRes = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`);
      const {
        n_peers: platformPeers,
      } = await tenderdashNetInfoRes.json();

      tenderdash.version = version;
      tenderdash.lastBlockHeight = latest_block_height;
      tenderdash.catchingUp = catching_up;
      tenderdash.peers = platformPeers;
      tenderdash.network = network;
      tenderdash.latestAppHash = latest_app_hash;
    } catch (e) {
      if (e.name !== 'FetchError') {
        throw e;
      }
    }
  } else {
    status = ServiceStatusEnum.wait_for_core;
  }

  return {
    status,
    httpService,
    httpPort,
    httpPortState,
    gRPCService,
    gRPCPort,
    gRPCPortState,
    p2pService,
    p2pPort,
    p2pPortState,
    rpcService,
    coreIsSynced,
    tenderdash,
  };
};

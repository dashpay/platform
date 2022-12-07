const fetch = require('node-fetch');
const determineStatus = require('../determineStatus');
const ServiceStatusEnum = require('../../enums/serviceStatus');
const providers = require('../providers');
const DockerStatusEnum = require("../../enums/dockerStatus");

module.exports = async (createRpcClient, dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const {
    result: {
      IsSynced: coreIsSynced,
    },
  } = await rpcClient.mnsync('status');

  const dockerStatus = await determineStatus.docker(dockerCompose, config, 'drive_tenderdash');

  let serviceStatus = determineStatus.platform(dockerStatus, coreIsSynced)

  const platform = {
    httpService: null,
    p2pService: null,
    gRPCService: null,
    rpcService: null,
    httpPortState: null,
    gRPCPortState: null,
    p2pPortState: null,
    tenderdash: {
      dockerStatus,
      serviceStatus,
      version: null,
      catchingUp: null,
      lastBlockHeight: null,
      latestAppHash: null,
      peers: null,
      network: null,
    }
  }

  // Collecting platform data fails if Tenderdash is waiting for core to sync
  try {
    const httpPort = config.get('platform.dapi.envoy.http.port');
    const gRPCPort = config.get('platform.dapi.envoy.grpc.port');
    const p2pPort = config.get('platform.drive.tenderdash.p2p.port');

    const [tenderdashStatusResponse, tenderdashNetInfoResponse,
      httpPortState, gRPCPortState, p2pPortState] = await Promise.all([
      fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`),
      fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`),
      providers.mnowatch.checkPortStatus(httpPort),
      providers.mnowatch.checkPortStatus(gRPCPort),
      providers.mnowatch.checkPortStatus(p2pPort)
    ])

    const [tenderdashNetInfo, tenderdashStatus] = await Promise.all([
      tenderdashNetInfoResponse.json(),
      tenderdashStatusResponse.json()
    ])

    platform.httpPortState = httpPortState
    platform.gRPCPortState = gRPCPortState
    platform.p2pPortState = p2pPortState

    platform.httpService = `${config.get('externalIp')}:${config.get('platform.dapi.envoy.http.port')}`;
    platform.gRPCService = `${config.get('externalIp')}:${config.get('platform.dapi.envoy.grpc.port')}`;
    platform.p2pService = `${config.get('externalIp')}:${config.get('platform.drive.tenderdash.p2p.port')}`;
    platform.rpcService = `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`;

    const {n_peers: platformPeers} = tenderdashNetInfo
    const {node_info, sync_info} = tenderdashStatus

    const {version, network} = node_info;
    const {catching_up, latest_block_height, latest_app_hash} = sync_info;

    platform.tenderdash.version = version;
    platform.tenderdash.lastBlockHeight = latest_block_height;
    platform.tenderdash.catchingUp = catching_up;
    platform.tenderdash.peers = platformPeers;
    platform.tenderdash.network = network;
    platform.tenderdash.latestAppHash = latest_app_hash;
  } catch (e) {
    if (e.name !== 'FetchError') {
      throw e;
    }

    platform.tenderdash.serviceStatus = ServiceStatusEnum.error
  }

  return platform
};

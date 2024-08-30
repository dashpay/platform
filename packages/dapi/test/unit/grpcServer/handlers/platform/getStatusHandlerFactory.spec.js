const EventEmitter = require('events');
const getStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getStatusHandlerFactory');

describe('getStatusHandlerFactory', () => {
  let blockchainListenerMock;
  let driveStatus;
  let driveStatusResponse;
  let driveClientMock;
  let tenderdashNetInfo;
  let tenderdashNetInfoResponse;
  let tenderdashStatus;
  let tenderdashStatusResponse;
  let tenderdashRpcClientMock;
  let getStatusHandler;

  beforeEach(function beforeEach() {
    blockchainListenerMock = new EventEmitter();

    this.sinon.spy(blockchainListenerMock);

    driveStatus = {
      version: {
        software: {
          drive: '1.1.1',
        },
        protocol: {
          drive: {
            current: 1,
            latest: 2,
          },
        },
      },
      chain: {
        coreChainLockedHeight: 1,
      },
      time: {
        block: 123,
        genesis: 100,
        epoch: 10,
      },
    };

    driveStatusResponse = {
      getV0() {
        return {
          toObject() {
            return driveStatus;
          },
        };
      },
    };

    driveClientMock = {
      getStatus: this.sinon.stub(),
    };

    tenderdashStatus = {
      node_info: {
        protocol_version: {
          p2p: '10',
          block: '14',
          app: '1',
        },
        id: '68c506d43816d1a8389c860c1d162be44d1e777c',
        ProTxHash: '85F15A31D3838293A9C1D72A1A0FA21E66110CE20878BD4C1024C4AE1D5BE824',
        network: 'dash-testnet-51',
        version: '1.2.0',
      },
      sync_info: {
        latest_block_hash: '17CF6008349C5C6E919BE05A9C53EC38565C0C5BF549AC38E204DEC8BF59729B',
        latest_app_hash: '86FE2247F22C1AEB4E5138E97E6164D61C1E40481F8FF62D126700EDE0EC9B92',
        latest_block_height: '1117',
        latest_block_time: '2024-08-27T08:31:39.906Z',
        earliest_block_hash: '08FA02C27EC0390BA301E4FC7E3D7EADB350C8193E3E62A093689706E3A20BFA',
        earliest_app_hash: 'BF0CCB9CA071BA01AE6E67A0C090F97803D26D56D675DCD5131781CBCAC8EC8F',
        earliest_block_height: '1',
        earliest_block_time: '2024-07-19T01:40:09Z',
        max_peer_block_height: '1117',
        catching_up: false,
        total_synced_time: '0',
        remaining_time: '0',
        total_snapshots: '0',
        chunk_process_avg_time: '0',
        snapshot_height: '0',
        snapshot_chunks_count: '0',
        backfilled_blocks: '0',
        backfill_blocks_total: '0',
      },
    };

    tenderdashStatusResponse = {
      result: tenderdashStatus,
    };

    tenderdashNetInfo = {
      listening: true,
      n_peers: 8,
    };

    tenderdashNetInfoResponse = {
      result: tenderdashNetInfo,
    };

    tenderdashRpcClientMock = {
      request: this.sinon.stub(),
    };

    getStatusHandler = getStatusHandlerFactory(
      blockchainListenerMock,
      driveClientMock,
      tenderdashRpcClientMock,
    );
  });

  it('should respond with full status if drive and tenderdash are responding', async () => {
    driveClientMock.getStatus.resolves(driveStatusResponse);
    tenderdashRpcClientMock.request.withArgs('status').resolves(tenderdashStatusResponse);
    tenderdashRpcClientMock.request.withArgs('net_info').resolves(tenderdashNetInfoResponse);

    const response = await getStatusHandler();

    response.serializeBinary();
  });

  it('should respond with DAPI only status if drive and tenderdash are not responding', async () => {
    driveClientMock.getStatus.rejects(new Error('Connection failed'));
    tenderdashRpcClientMock.request.rejects(new Error('Connection failed'));

    const response = await getStatusHandler();

    response.serializeBinary();
  });

  it('should respond with partial status if drive is not responding', async () => {
    driveClientMock.getStatus.rejects(new Error('Connection failed'));
    tenderdashRpcClientMock.request.withArgs('status').resolves(tenderdashStatusResponse);
    tenderdashRpcClientMock.request.withArgs('net_info').resolves(tenderdashNetInfoResponse);

    const response = await getStatusHandler();

    response.serializeBinary();
  });

  it('should respond with partial status if tenderdash is not responding', async () => {
    driveClientMock.getStatus.resolves(driveStatusResponse);
    tenderdashRpcClientMock.request.rejects(new Error('Connection failed'));

    const response = await getStatusHandler();

    response.serializeBinary();
  });
});

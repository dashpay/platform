const outputStatusOverviewFactory = require('../../src/status/outputStatusOverviewFactory')

const sinon = require('sinon')

describe('Dashmate', () => {
  let outputStatusOverview

  let dockerComposeMock
  let mockRpcClient

  beforeEach(async function it() {
    dockerComposeMock = {isServiceRunning: sinon.stub(), docker: {getContainer: sinon.stub()}}
    mockRpcClient = {mnsync: sinon.stub(), getNetworkInfo: sinon.stub(), getBlockchainInfo: sinon.stub()}
    const createRpcClient = () => (mockRpcClient)

    outputStatusOverview = outputStatusOverviewFactory(dockerComposeMock, createRpcClient)
  });

  it('should basically work', async () => {
    const mockConfig = {network: 'test', 'core.masternode.enable': false, 'platform.drive.tenderdash.rpc.port': 8080}
    const mockMNSync = {
      "AssetID": 999,
      "AssetName": "MASTERNODE_SYNC_FINISHED",
      "AssetStartTime": 1507662300,
      "Attempt": 0,
      "IsBlockchainSynced": true,
      "IsSynced": true,
    }

    const mockGetNetworkInfo = {
      "version": 170003,
      "buildversion": "v0.17.0.3-649273e70",
      "subversion": "/Dash Core:0.17.0.3/",
      "protocolversion": 70220,
      "localservices": "0000000000000445",
      "localservicesnames": [
        "NETWORK",
        "BLOOM",
        "COMPACT_FILTERS",
        "NETWORK_LIMITED"
      ],
      "localrelay": true,
      "timeoffset": 0,
      "networkactive": true,
      "connections": 8,
      "socketevents": "epoll",
      "networks": [
        {
          "name": "ipv4",
          "limited": false,
          "reachable": true,
          "proxy": "",
          "proxy_randomize_credentials": false
        },
        {
          "name": "ipv6",
          "limited": false,
          "reachable": true,
          "proxy": "",
          "proxy_randomize_credentials": false
        },
        {
          "name": "onion",
          "limited": true,
          "reachable": false,
          "proxy": "",
          "proxy_randomize_credentials": false
        },
        {
          "name": "",
          "limited": false,
          "reachable": true,
          "proxy": "",
          "proxy_randomize_credentials": false
        },
        {
          "name": "",
          "limited": false,
          "reachable": true,
          "proxy": "",
          "proxy_randomize_credentials": false
        }
      ],
      "relayfee": 0.00001000,
      "incrementalfee": 0.00001000,
      "localaddresses": [
      ],
      "warnings": "Warning: unknown new rules activated (versionbit 3)"
    }
    const mockGetBlockchainInfo = {
      "chain": "test",
      "blocks": 292973,
      "headers": 292973,
      "bestblockhash": "0000020029bcac549a6e7b7e488d9ca8af518d4c0aae8073cd364c70ca29be6e",
      "difficulty": 0.0002441371325370145,
      "mediantime": 1586975225,
      "verificationprogress": 0.9999983278651547,
      "initialblockdownload": false,
      "chainwork": "00000000000000000000000000000000000000000000000001e6f68a064798f8",
      "size_on_disk": 1186147401,
      "pruned": false,
      "softforks": [
        {
          "id": "bip34",
          "version": 2,
          "reject": {
            "status": true
          }
        },
        {
          "id": "bip66",
          "version": 3,
          "reject": {
            "status": true
          }
        },
        {
          "id": "bip65",
          "version": 4,
          "reject": {
            "status": true
          }
        }
      ],
      "bip9_softforks": {
        "csv": {
          "status": "active",
          "startTime": 1544655600,
          "timeout": 1576191600,
          "since": 8064
        },
        "dip0001": {
          "status": "active",
          "startTime": 1544655600,
          "timeout": 1576191600,
          "since": 4400
        },
        "bip147": {
          "status": "active",
          "startTime": 1544655600,
          "timeout": 1576191600,
          "since": 4300
        },
        "dip0003": {
          "status": "active",
          "startTime": 1544655600,
          "timeout": 1576191600,
          "since": 7000
        },
        "dip0008": {
          "status": "active",
          "startTime": 1553126400,
          "timeout": 1584748800,
          "since": 78800
        }
      },
      "warnings": "Warning: unknown new rules activated (versionbit 3)"
    }

    mockRpcClient.mnsync.resolves({result: mockMNSync})
    mockRpcClient.getNetworkInfo.resolves({result: mockGetNetworkInfo})
    mockRpcClient.getBlockchainInfo.resolves({result: mockGetBlockchainInfo})

    const config = {get: (path) => mockConfig[path], toEnvs: sinon.stub()}

    dockerComposeMock.isServiceRunning.resolves(true);

    const status = await outputStatusOverview(config, 'json')

    console.log(status)

    expect(status).to.exist()
  });
});

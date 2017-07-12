const bootstrap = ['/ip4/155.94.181.16/tcp/4001/Qmdgv66suuEpdqoa3J1sem7VzwNzavqsi2wR5WPiAcrxLM']

module.exports = {
  daemon1: {
    IpfsDataDir: '/tmp/orbit-db-tests-1',
    Addresses: {
      API: '/ip4/127.0.0.0/tcp/0',
      Swarm: ['/ip4/0.0.0.0/tcp/0'], //'/ip4/173.212.223.26/tcp/0'
      Gateway: '/ip4/0.0.0.0/tcp/0'
    },
    Bootstrap: bootstrap,
    Discovery: {
      MDNS: {
        Enabled: true,
        Interval: 10
      },
      webRTCStar: {
        Enabled: false
      }
    }
  },
  daemon2: {
    IpfsDataDir: '/tmp/orbit-db-tests-2',
    Addresses: {
      API: '/ip4/127.0.0.0/tcp/0',
      Swarm: ['/ip4/0.0.0.0/tcp/0'], // '/ip4/155.94.181.16/tcp/4001'
      Gateway: '/ip4/0.0.0.0/tcp/0'
    },
    Bootstrap: bootstrap,
    Discovery: {
      MDNS: {
        Enabled: true,
        Interval: 10
      },
      webRTCStar: {
        Enabled: false
      }
    }
  }
}

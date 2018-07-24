const config = {
  Api: {
    port: 3000,
  },
  MNListUpdateInterval: 60000,
  quorumUpdateInterval: 60000,
  DAPIDNSSeeds: [
    {
      vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
      status: 'ENABLED',
      rank: 1,
      ip: '127.0.0.1',
      protocol: 70208,
      payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
      activeseconds: 1073078,
      lastseen: 1516291362,
    },
    // Uncomment if an extra node is required (server owned by Pierre)
    // {
    //   vin: '54754314335419cc04ef09295ff7765c8062a6123486aed55fd7e9b04f300b13-0',
    //   status: 'ENABLED',
    //   rank: 1,
    //   ip: '173.212.223.26',
    //   protocol: 70208,
    //   payee: 'ycn5RWc4Ruo35FTS8bJwugVyCEkfVcrw9a',
    //   activeseconds: 1073078,
    //   lastseen: 1516291362,
    // },
  ],
};

module.exports = config;

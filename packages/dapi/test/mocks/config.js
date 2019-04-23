module.exports = {
  getConfigFixture() {
    return {
      insightUri: '123',
      dashcore: {
        p2p: {
          host: '123',
          port: '123',
        },
        rpc: {
          host: '123',
          port: '123',
        },
        zmq: {
          port: '123',
          host: '123',
        },
      },
      drive: {
        host: '123',
        port: '123',
      },
      rpcServer: {
        port: '123',
      },
      grpcServer: {
        port: '123',
      },
    };
  },
};

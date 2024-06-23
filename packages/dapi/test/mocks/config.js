module.exports = {
  getConfigFixture() {
    return {
      dashcore: {
        rpc: {
          host: '123',
          port: '123',
        },
        zmq: {
          port: '123',
          host: '123',
        },
      },
      tendermintCore: {
        host: '123',
        port: '123',
      },
      rpcServer: {
        port: '123',
      },
      grpcServer: {
        port: '123',
      },
      txFilterStream: {
        grpcServer: {
          port: '123',
        },
      },
    };
  },
};

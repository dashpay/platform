const {
  v0: {
    MasternodeListRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const GrpcTransport = require('../transport/GrpcTransport/GrpcTransport');
const createGrpcTransportError = require('../transport/GrpcTransport/createGrpcTransportError');
const ReconnectableStream = require('../transport/ReconnectableStream');

/**
 * Creates continues masternode list stream
 *
 * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
 * @param {ListDAPIAddressProvider} listDAPIAddressProvider
 * @param {Object} options
 * @return {function(...[*]): Promise<ReconnectableStream>}
 */
function createMasternodeListStreamFactory(
  createDAPIAddressProviderFromOptions,
  listDAPIAddressProvider,
  options,
) {
  const grpcTransport = new GrpcTransport(
    createDAPIAddressProviderFromOptions,
    listDAPIAddressProvider,
    createGrpcTransportError,
    options,
  );

  return ReconnectableStream
    .create(
      () => grpcTransport.request(
        CorePromiseClient,
        'subscribeToMasternodeList',
        new MasternodeListRequest(),
        {
          timeout: undefined,
          autoReconnectInterval: 0,
        },
      ),
      {
        maxRetriesOnError: -1,
      },
    );
}

module.exports = createMasternodeListStreamFactory;

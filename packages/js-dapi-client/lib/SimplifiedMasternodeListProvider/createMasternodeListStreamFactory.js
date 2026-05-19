import dapiGrpc from '@dashevo/dapi-grpc';

import GrpcTransport from '../transport/GrpcTransport/GrpcTransport.js';
import createGrpcTransportError from '../transport/GrpcTransport/createGrpcTransportError.js';
import ReconnectableStream from '../transport/ReconnectableStream.js';

const {
  v0: {
    MasternodeListRequest,
    CorePromiseClient,
  },
} = dapiGrpc;

/**
 * Creates continues masternode list stream
 * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
 * @param {ListDAPIAddressProvider} listDAPIAddressProvider
 * @param {object} options
 * @returns {function(...[*]): Promise<ReconnectableStream>}
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
        logger: options.logger,
      },
    );
}

export default createMasternodeListStreamFactory;

import DAPIClient from './DAPIClient.js';
import NotFoundError from './transport/GrpcTransport/errors/NotFoundError.js';
import BlockHeadersProvider from './BlockHeadersProvider/BlockHeadersProvider.js';
import * as bytes from './utils/bytes.js';

DAPIClient.Errors = {
  NotFoundError,
};

DAPIClient.BlockHeadersProvider = BlockHeadersProvider;
DAPIClient.bytes = bytes;

export default DAPIClient;
export { NotFoundError, BlockHeadersProvider, bytes };

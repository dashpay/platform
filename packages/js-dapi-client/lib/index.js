import DAPIClient from './DAPIClient.js';
import NotFoundError from './transport/GrpcTransport/errors/NotFoundError.js';
import BlockHeadersProvider from './BlockHeadersProvider/BlockHeadersProvider.js';

DAPIClient.Errors = {
  NotFoundError,
};

DAPIClient.BlockHeadersProvider = BlockHeadersProvider;

export default DAPIClient;
export { NotFoundError, BlockHeadersProvider };

import logger from '../../../logger/index.js';

export default async function getBlockHeaderByHash(blockHash) {
  logger.silly(`DAPIClient.getBlockHeaderByHash[${blockHash}]`);

  return (await this.getBlockByHash(blockHash)).header;
};

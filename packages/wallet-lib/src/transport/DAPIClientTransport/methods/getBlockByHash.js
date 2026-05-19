import dashcore from '@dashevo/dashcore-lib';
const { Block } = dashcore;
import logger from '../../../logger/index.js';

export default async function getBlockByHash(blockHash) {
  logger.silly(`DAPIClient.getBlockByHash[${blockHash}]`);

  return new Block(await this.client.core.getBlockByHash(blockHash));
};

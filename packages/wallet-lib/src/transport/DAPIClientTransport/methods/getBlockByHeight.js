import dashcore from '@dashevo/dashcore-lib';
const { Block } = dashcore;
import logger from '../../../logger/index.js';

export default async function getBlockByHeight(height) {
  logger.silly(`DAPIClient.getBlockByHeight[${height}]`);

  return new Block(await this.client.core.getBlockByHeight(height));
};

import logger from '../../../logger/index.js';

export default async function getBlockHeaderByHeight(blockHeight) {
  logger.silly(`DAPIClient.getBlockHeaderByHeight[${blockHeight}]`);
  return (await this.getBlockByHeight(blockHeight)).header;
};

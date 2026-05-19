export default async function getBlockHeaderByHeight(blockHeight) {
  return (await this.getBlockByHeight(blockHeight)).header;
};
